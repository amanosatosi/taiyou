//! The only unsafe module: narrow FreeType/HarfBuzz ownership, no public handles.
#![allow(unsafe_code)]
use crate::shape::{NativeRequest, RawGlyph, RawRun};
use freetype_sys as ft;
use harfbuzz_sys as hb;
use std::{ffi::CStr, marker::PhantomData, ptr, rc::Rc, sync::Arc};
use taiyou_types::*;

struct HbBlob {
    raw: *mut hb::hb_blob_t,
    _bytes: Arc<[u8]>,
}
impl Drop for HbBlob {
    fn drop(&mut self) {
        /* SAFETY: one owned constructor reference, bytes still live. */
        unsafe {
            hb::hb_blob_destroy(self.raw);
        }
    }
}
struct HbFace {
    raw: *mut hb::hb_face_t,
    _blob: HbBlob,
}
impl Drop for HbFace {
    fn drop(&mut self) {
        /* SAFETY: owned face released before its retained blob. */
        unsafe {
            hb::hb_face_destroy(self.raw);
        }
    }
}
struct HbFont {
    raw: *mut hb::hb_font_t,
    face: HbFace,
}
impl Drop for HbFont {
    fn drop(&mut self) {
        /* SAFETY: owned font released before the retained face. */
        unsafe {
            hb::hb_font_destroy(self.raw);
        }
    }
}
struct HbBuffer(*mut hb::hb_buffer_t);
impl Drop for HbBuffer {
    fn drop(&mut self) {
        /* SAFETY: unique per-call buffer, copied results outlive it. */
        unsafe {
            hb::hb_buffer_destroy(self.0);
        }
    }
}

impl FontContext {
    pub(crate) fn shape_raw(
        &mut self,
        blob: &FontBlob,
        request: &NativeRequest<'_>,
        limits: &Limits,
    ) -> Result<RawRun, FontError> {
        self.check_bytes(blob.bytes())?;
        crate::catalog::check_blob_identity(blob)?;
        let invalid = || {
            FontError::new(
                ErrorCode::InvalidArgument,
                "invalid native shaping text/context/options",
            )
        };
        let limited = || {
            FontError::new(
                ErrorCode::LimitExceeded,
                "native shaping reservation exceeded",
            )
        };
        let failed =
            || FontError::new(ErrorCode::NativeFailure, "HarfBuzz OpenType shaping failed");
        let context = request
            .text
            .get(request.context.clone())
            .ok_or_else(invalid)?;
        if request.text.len() > u32::MAX as usize
            || request.range.start < request.context.start
            || request.range.end > request.context.end
            || request.text.get(request.range.clone()).is_none()
            || request.language.len() > 63
            || request.language.is_empty()
            || !request
                .language
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        {
            return Err(invalid());
        }
        let length = i32::try_from(context.len()).map_err(|_| limited())?;
        if context.len() as u64 > limits.text_bytes_per_event() {
            return Err(limited());
        }
        let charge = (blob.bytes().len() as u64)
            .checked_mul(4)
            .and_then(|n| n.checked_add((context.len() as u64).checked_mul(256)?))
            .and_then(|n| {
                n.checked_add(
                    limits
                        .glyphs_per_event()
                        .checked_mul(3 * std::mem::size_of::<RawGlyph>() as u64)?,
                )
            })
            .and_then(|n| n.checked_add(4096))
            .ok_or_else(limited)?;
        if charge > limits.transient_bytes() || charge > self.limits.transient_bytes() {
            return Err(limited());
        }
        let font_len = u32::try_from(blob.bytes().len()).map_err(|_| limited())?;
        let bytes = blob.shared_bytes();
        // SAFETY: immutable bytes have checked extent and remain owned by HbBlob.
        // Every constructor is checked and RAII unwinds earlier handles on error.
        unsafe {
            let raw = hb::hb_blob_create(
                bytes.as_ptr().cast(),
                font_len,
                hb::HB_MEMORY_MODE_READONLY,
                ptr::null_mut(),
                None,
            );
            if raw.is_null() {
                return Err(failed());
            }
            let owned_blob = HbBlob { raw, _bytes: bytes };
            if hb::hb_blob_get_length(raw) != font_len {
                return Err(FontError::new(
                    ErrorCode::AllocationFailed,
                    "HarfBuzz blob allocation failed",
                ));
            }
            if blob.id().face_index() >= hb::hb_face_count(raw) {
                return Err(invalid());
            }
            let raw_face = hb::hb_face_create(raw, blob.id().face_index());
            if raw_face.is_null() {
                return Err(failed());
            }
            let face = HbFace {
                raw: raw_face,
                _blob: owned_blob,
            };
            let upem = hb::hb_face_get_upem(face.raw);
            let scale = i32::try_from(upem).map_err(|_| failed())?;
            if scale <= 0 || hb::hb_face_get_glyph_count(face.raw) == 0 {
                return Err(failed());
            }
            let raw_font = hb::hb_font_create(face.raw);
            if raw_font.is_null() {
                return Err(failed());
            }
            let font = HbFont {
                raw: raw_font,
                face,
            };
            if font.raw == hb::hb_font_get_empty() {
                return Err(FontError::new(
                    ErrorCode::AllocationFailed,
                    "HarfBuzz font allocation failed",
                ));
            }
            hb::hb_ot_font_set_funcs(font.raw);
            hb::hb_font_set_scale(font.raw, scale, scale);
            // hb_font_extents_t is a public POD integer struct, including padding.
            let mut extents: hb::hb_font_extents_t = std::mem::zeroed();
            if hb::hb_font_get_h_extents(font.raw, &mut extents) == 0 {
                return Err(failed());
            }
            let raw_buffer = hb::hb_buffer_create();
            if raw_buffer.is_null() {
                return Err(failed());
            }
            let buffer = HbBuffer(raw_buffer);
            if hb::hb_buffer_pre_allocate(
                buffer.0,
                u32::try_from(request.range.len()).map_err(|_| limited())?,
            ) == 0
            {
                return Err(FontError::new(
                    ErrorCode::AllocationFailed,
                    "HarfBuzz buffer allocation failed",
                ));
            }
            hb::hb_buffer_set_cluster_level(
                buffer.0,
                hb::HB_BUFFER_CLUSTER_LEVEL_MONOTONE_GRAPHEMES,
            );
            hb::hb_buffer_set_direction(
                buffer.0,
                if request.rtl {
                    hb::HB_DIRECTION_RTL
                } else {
                    hb::HB_DIRECTION_LTR
                },
            );
            hb::hb_buffer_set_script(buffer.0, hb::hb_script_from_iso15924_tag(request.script));
            hb::hb_buffer_set_language(
                buffer.0,
                hb::hb_language_from_string(
                    request.language.as_ptr().cast(),
                    request.language.len() as i32,
                ),
            );
            let flags = if request.range.start == request.context.start {
                hb::HB_BUFFER_FLAG_BOT
            } else {
                0
            } | if request.range.end == request.context.end {
                hb::HB_BUFFER_FLAG_EOT
            } else {
                0
            };
            hb::hb_buffer_set_flags(buffer.0, flags);
            hb::hb_buffer_add_utf8(
                buffer.0,
                context.as_ptr().cast(),
                length,
                (request.range.start - request.context.start) as u32,
                request.range.len() as i32,
            );
            let feature = |tag, value| hb::hb_feature_t {
                tag,
                value,
                start: 0,
                end: u32::MAX,
            };
            let features = [
                feature(u32::from_be_bytes(*b"kern"), 1),
                feature(u32::from_be_bytes(*b"liga"), u32::from(request.ligatures)),
                feature(u32::from_be_bytes(*b"clig"), u32::from(request.ligatures)),
            ];
            let shapers = [c"ot".as_ptr(), ptr::null()];
            if hb::hb_shape_full(
                font.raw,
                buffer.0,
                features.as_ptr(),
                features.len() as u32,
                shapers.as_ptr(),
            ) == 0
                || hb::hb_buffer_allocation_successful(buffer.0) == 0
            {
                return Err(failed());
            }
            let count = hb::hb_buffer_get_length(buffer.0);
            if u64::from(count) > limits.glyphs_per_event() {
                return Err(limited());
            }
            let (mut info_count, mut pos_count) = (0, 0);
            let info = hb::hb_buffer_get_glyph_infos(buffer.0, &mut info_count);
            let positions = hb::hb_buffer_get_glyph_positions(buffer.0, &mut pos_count);
            if info_count != count
                || pos_count != count
                || (count != 0 && (info.is_null() || positions.is_null()))
            {
                return Err(failed());
            }
            let mut glyphs = Vec::new();
            glyphs.try_reserve_exact(count as usize).map_err(|_| {
                FontError::new(ErrorCode::AllocationFailed, "owned glyph allocation failed")
            })?;
            for i in 0..count as usize {
                // Pointers address exactly count native-owned POD entries; they
                // are read only until all values are copied before buffer drop.
                let g = &*info.add(i);
                let p = &*positions.add(i);
                let cluster = (request.context.start as u32)
                    .checked_add(g.cluster)
                    .ok_or_else(limited)?;
                if (cluster as usize) < request.range.start
                    || cluster as usize >= request.range.end
                    || !request.text.is_char_boundary(cluster as usize)
                {
                    return Err(failed());
                }
                glyphs.push(RawGlyph {
                    glyph: g.codepoint,
                    cluster,
                    advance_x: p.x_advance,
                    advance_y: p.y_advance,
                    offset_x: p.x_offset,
                    offset_y: p.y_offset,
                });
            }
            // Keep ownership explicit until after the last native read.
            let _ = font.face.raw;
            Ok(RawRun {
                glyphs,
                upem,
                ascent: extents.ascender,
                descent: extents.descender,
                line_gap: extents.line_gap,
            })
        }
    }
}

#[cfg(target_os = "linux")]
#[link(name = "stdc++")]
unsafe extern "C" {}
#[cfg(target_vendor = "apple")]
#[link(name = "c++")]
unsafe extern "C" {}

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
#[cfg(test)]
std::thread_local! {
    static LIVE: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
struct Library(ft::FT_Library);
impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: this unique initialized library outlives every borrowed Face.
        unsafe {
            ft::FT_Done_FreeType(self.0);
        }
        #[cfg(test)]
        LIVE.with(|n| {
            let (libraries, faces) = n.get();
            n.set((libraries - 1, faces));
        });
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
        #[cfg(test)]
        LIVE.with(|n| {
            let (libraries, faces) = n.get();
            n.set((libraries, faces - 1));
        });
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
    limits: Limits,
    _local: PhantomData<Rc<()>>,
}
impl FontContext {
    pub fn new() -> Result<Self, FontError> {
        Self::with_limits(Limits::default())
    }
    pub fn with_limits(limits: Limits) -> Result<Self, FontError> {
        Self::with_version(limits, (2, 14, 3))
    }
    fn with_version(limits: Limits, expected: (i32, i32, i32)) -> Result<Self, FontError> {
        let (mut hb_major, mut hb_minor, mut hb_patch) = (0, 0, 0);
        // SAFETY: three valid integer out slots; no native handles are involved.
        unsafe {
            hb::hb_version(&mut hb_major, &mut hb_minor, &mut hb_patch);
        }
        if (hb_major, hb_minor, hb_patch) != (14, 5, 0) {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "HarfBuzz does not match pinned 14.5.0 recipe",
            ));
        }
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
        #[cfg(test)]
        LIVE.with(|n| {
            let (libraries, faces) = n.get();
            n.set((libraries + 1, faces));
        });
        let (mut major, mut minor, mut patch) = (0, 0, 0);
        // SAFETY: initialized live library and valid local integer out slots.
        unsafe {
            FT_Library_Version(library.0, &mut major, &mut minor, &mut patch);
        }
        if (major, minor, patch) != expected {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "FreeType does not match pinned 2.14.3 recipe",
            ));
        }
        Ok(Self {
            library,
            version: (major, minor, patch),
            limits,
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
        #[cfg(test)]
        LIVE.with(|n| {
            let (libraries, faces) = n.get();
            n.set((libraries, faces + 1));
        });
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
        if index > 0xffff || !u32::try_from(count).is_ok_and(|count| index < count) {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "collection face index out of range or named-instance selector",
            ));
        }
        drop(probe);
        let face = self.open(bytes, ft::FT_Long::from(index as u16))?;
        read(&face)
    }
    pub(crate) fn check_bytes(&self, bytes: &[u8]) -> Result<(), FontError> {
        let byte_limit = bytes.len();
        if byte_limit == 0 {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "font bytes are empty",
            ));
        }
        let cost = (byte_limit as u64)
            .checked_mul(12)
            .and_then(|n| n.checked_add(64 * 1024));
        if byte_limit as u64 > self.limits.font_bytes()
            || cost.is_none_or(|n| n > self.limits.transient_bytes())
        {
            return Err(FontError::new(
                ErrorCode::LimitExceeded,
                "native intake reservation exceeded",
            ));
        }
        Ok(())
    }
    pub(crate) fn inspect(&mut self, bytes: Arc<[u8]>, index: u32) -> Result<Metadata, FontError> {
        self.check_bytes(&bytes)?;
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
        self.check_bytes(blob.bytes())?;
        crate::catalog::check_blob_identity(blob)?;
        Ok(self
            .inspect(blob.shared_bytes(), blob.id().face_index())?
            .glyphs)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_library_initialization_finishes_native_owner() {
        assert_eq!(LIVE.with(|n| n.get()), (0, 0));
        assert!(FontContext::with_version(Limits::default(), (0, 0, 0)).is_err());
        assert_eq!(LIVE.with(|n| n.get()), (0, 0));
        let context = FontContext::new().unwrap();
        assert_eq!(LIVE.with(|n| n.get()), (1, 0));
        drop(context);
        assert_eq!(LIVE.with(|n| n.get()), (0, 0));
    }
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
            assert_eq!(LIVE.with(|n| n.get()), (1, 0));
            assert_eq!(context.inspect(bytes.clone(), 0).unwrap().glyphs, 2478);
            assert_eq!(LIVE.with(|n| n.get()), (1, 0));
        }
        drop(context);
        assert_eq!(LIVE.with(|n| n.get()), (0, 0));
    }
}
