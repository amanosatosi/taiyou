//! The only unsafe module: narrow FreeType ownership, no public native handles.
#![allow(unsafe_code)]
use freetype_sys as ft;
use std::{ffi::CStr, marker::PhantomData, ptr, rc::Rc, sync::Arc};
use taiyou_types::*;

// Stable diagnostic/SFNT APIs omitted from freetype-sys 0.23.0. These declarations
// and the small TT_OS2 prefix match the pinned 2.14.3 public headers, not a font
// parser. Their ABI and supported native versions require independent review.
#[repr(C)]
struct SfntName {
    platform: u16,
    encoding: u16,
    language: u16,
    name: u16,
    string: *mut u8,
    length: u32,
}
#[repr(C)]
struct Os2Prefix {
    version: u16,
    average_width: i16,
    weight: u16,
}
unsafe extern "C" {
    fn FT_Library_Version(
        library: ft::FT_Library,
        major: *mut ft::FT_Int,
        minor: *mut ft::FT_Int,
        patch: *mut ft::FT_Int,
    );
    fn FT_Get_Sfnt_Name_Count(face: ft::FT_Face) -> u32;
    fn FT_Get_Sfnt_Name(face: ft::FT_Face, index: u32, name: *mut SfntName) -> ft::FT_Error;
    fn FT_Get_Sfnt_Table(face: ft::FT_Face, tag: i32) -> *mut std::ffi::c_void;
}
fn native_error(code: ft::FT_Error, detail: &str) -> FontError {
    FontError::new(ErrorCode::NativeFailure, detail).with_native_code(i64::from(code))
}
struct Library(ft::FT_Library);
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: this unique initialized library outlives every borrowed Face.
        unsafe {
            ft::FT_Done_FreeType(self.0);
        }
    }
}
struct Face<'a> {
    raw: ft::FT_Face,
    _library: &'a Library,
    _bytes: Arc<[u8]>,
}
impl Drop for Face<'_> {
    fn drop(&mut self) {
        // SAFETY: successful memory-face constructor; bytes/library still live.
        unsafe {
            ft::FT_Done_Face(self.raw);
        }
    }
}
pub(crate) struct Metadata {
    pub family: String,
    pub weight: u16,
    pub italic: bool,
    pub glyphs: u32,
}
/// Thread-local native owner. It cannot be sent or shared; handles never escape.
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<taiyou_font::FontContext>();
/// ```
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<taiyou_font::FontContext>();
/// ```
pub struct FontContext {
    library: Library,
    version: (i32, i32, i32),
    _local: PhantomData<Rc<()>>,
}
impl FontContext {
    pub fn new() -> Result<Self, FontError> {
        let mut raw = ptr::null_mut();
        // SAFETY: valid out slot, owned only after successful initialization.
        let code = unsafe { ft::FT_Init_FreeType(&mut raw) };
        if code != 0 {
            return Err(native_error(code, "FT_Init_FreeType failed"));
        }
        if raw.is_null() {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "FreeType returned a null library",
            ));
        }
        let library = Library(raw);
        let (mut major, mut minor, mut patch) = (0, 0, 0);
        // SAFETY: initialized live library and valid local integer out slots.
        unsafe {
            FT_Library_Version(library.0, &mut major, &mut minor, &mut patch);
        }
        if (major, minor, patch) != (2, 14, 3) {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "FreeType does not match pinned 2.14.3 recipe",
            ));
        }
        Ok(Self {
            library,
            version: (major, minor, patch),
            _local: PhantomData,
        })
    }
    pub const fn version(&self) -> (i32, i32, i32) {
        self.version
    }
    fn open<'a>(&'a self, bytes: Arc<[u8]>, index: ft::FT_Long) -> Result<Face<'a>, FontError> {
        let length = ft::FT_Long::try_from(bytes.len())
            .map_err(|_| FontError::new(ErrorCode::LimitExceeded, "native font length overflow"))?;
        let mut raw = ptr::null_mut();
        // SAFETY: immutable checked-length bytes retained by Face, live library,
        // valid out slot. A failed constructor publishes no owned face.
        let code = unsafe {
            ft::FT_New_Memory_Face(self.library.0, bytes.as_ptr(), length, index, &mut raw)
        };
        if code != 0 {
            return Err(native_error(code, "FT_New_Memory_Face failed"));
        }
        if raw.is_null() {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "FreeType returned a null face",
            ));
        }
        Ok(Face {
            raw,
            _library: &self.library,
            _bytes: bytes,
        })
    }
    fn with_face<T>(
        &mut self,
        bytes: Arc<[u8]>,
        index: u32,
        read: impl FnOnce(&Face<'_>) -> Result<T, FontError>,
    ) -> Result<T, FontError> {
        let probe = self.open(bytes.clone(), -1)?;
        // SAFETY: live initialized probe returned by FreeType.
        let count = unsafe { (*probe.raw).num_faces };
        if index > 0xffff || i64::from(index) >= i64::from(count) {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "collection face index out of range or named-instance selector",
            ));
        }
        drop(probe);
        let face = self.open(bytes, ft::FT_Long::from(index as u16))?;
        read(&face)
    }
    pub(crate) fn inspect(&mut self, bytes: Arc<[u8]>, index: u32) -> Result<Metadata, FontError> {
        let byte_limit = bytes.len();
        self.with_face(bytes, index, |face| {
            // SAFETY: live face, native APIs guarantee successful SFNT records
            // reference face-owned memory. Copy all names before dropping face.
            unsafe {
                let count = FT_Get_Sfnt_Name_Count(face.raw);
                if u64::from(count) > u64::try_from(byte_limit).unwrap_or(0) {
                    return Err(FontError::new(
                        ErrorCode::LimitExceeded,
                        "SFNT name count exceeds font bytes",
                    ));
                }
                let mut best: Option<((u8, u8, u16), String)> = None;
                for i in 0..count {
                    let mut name = SfntName {
                        platform: 0,
                        encoding: 0,
                        language: 0,
                        name: 0,
                        string: ptr::null_mut(),
                        length: 0,
                    };
                    let code = FT_Get_Sfnt_Name(face.raw, i, &mut name);
                    if code != 0 {
                        return Err(native_error(code, "FT_Get_Sfnt_Name failed"));
                    }
                    if !matches!(name.name, 1 | 16)
                        || !(name.platform == 0
                            || name.platform == 3 && matches!(name.encoding, 0 | 1 | 10))
                    {
                        continue;
                    }
                    let length = usize::try_from(name.length).map_err(|_| {
                        FontError::new(ErrorCode::LimitExceeded, "SFNT name length overflow")
                    })?;
                    if length > byte_limit || length % 2 != 0 || length > 0 && name.string.is_null()
                    {
                        return Err(FontError::new(
                            ErrorCode::FontUnavailable,
                            "invalid SFNT Unicode name",
                        ));
                    }
                    if length == 0 {
                        continue;
                    }
                    let raw = std::slice::from_raw_parts(name.string, length);
                    let mut units = Vec::new();
                    units.try_reserve_exact(length / 2).map_err(|_| {
                        FontError::new(ErrorCode::AllocationFailed, "name allocation failed")
                    })?;
                    units.extend(
                        raw.chunks_exact(2)
                            .map(|b| u16::from_be_bytes([b[0], b[1]])),
                    );
                    let family = String::from_utf16(&units).map_err(|_| {
                        FontError::new(ErrorCode::FontUnavailable, "invalid UTF-16 font family")
                    })?;
                    if family.is_empty() {
                        continue;
                    }
                    let rank = (
                        u8::from(name.name != 16),
                        u8::from(name.language != 0x409),
                        name.language,
                    );
                    if best.as_ref().is_none_or(|(old, _)| rank < *old) {
                        best = Some((rank, family));
                    }
                }
                let family = if let Some((_, family)) = best {
                    family
                } else {
                    let p = (*face.raw).family_name;
                    if p.is_null() {
                        return Err(FontError::new(
                            ErrorCode::FontUnavailable,
                            "font family unavailable",
                        ));
                    }
                    CStr::from_ptr(p)
                        .to_str()
                        .map_err(|_| {
                            FontError::new(ErrorCode::FontUnavailable, "non-UTF-8 family fallback")
                        })?
                        .to_owned()
                };
                let os2 = FT_Get_Sfnt_Table(face.raw, 2);
                let weight = if os2.is_null() {
                    if (*face.raw).style_flags & ft::FT_STYLE_FLAG_BOLD != 0 {
                        700
                    } else {
                        400
                    }
                } else {
                    (*os2.cast::<Os2Prefix>()).weight
                };
                let glyphs = u32::try_from((*face.raw).num_glyphs).map_err(|_| {
                    FontError::new(ErrorCode::FontUnavailable, "invalid glyph count")
                })?;
                Ok(Metadata {
                    family,
                    weight,
                    italic: (*face.raw).style_flags & ft::FT_STYLE_FLAG_ITALIC != 0,
                    glyphs,
                })
            }
        })
    }
    pub fn glyph_count(&mut self, blob: &FontBlob) -> Result<u32, FontError> {
        Ok(self
            .inspect(blob.shared_bytes(), blob.id().face_index())?
            .glyphs)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failure_after_face_initialization_releases_face_and_library_remains_usable() {
        let bytes: Arc<[u8]> =
            std::fs::read(std::env::var("TAIYOU_TEST_FONT").expect("native CI font required"))
                .unwrap()
                .into();
        let mut context = FontContext::new().unwrap();
        for _ in 0..32 {
            let error = context
                .with_face(bytes.clone(), 0, |_| {
                    Err::<(), _>(FontError::new(
                        ErrorCode::NativeFailure,
                        "injected after successful face creation",
                    ))
                })
                .unwrap_err();
            assert_eq!(error.code(), ErrorCode::NativeFailure);
            assert_eq!(context.inspect(bytes.clone(), 0).unwrap().glyphs, 2478);
        }
    }
}
