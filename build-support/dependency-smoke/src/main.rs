//! Runner-only native binding audit, not a production font adapter.
use freetype_sys as ft;
use harfbuzz_sys as hb;
use std::ptr;
use unicode_script::{Script, UnicodeScript};

struct FreeType {
    library: ft::FT_Library,
    face: ft::FT_Face,
}
impl Drop for FreeType {
    fn drop(&mut self) {
        // SAFETY: both handles were checked after successful creation; face is
        // destroyed first, exactly once, while the library and font bytes live.
        unsafe {
            ft::FT_Done_Face(self.face);
            ft::FT_Done_FreeType(self.library);
        }
    }
}
struct HarfBuzz {
    blob: *mut hb::hb_blob_t,
    face: *mut hb::hb_face_t,
}
impl Drop for HarfBuzz {
    fn drop(&mut self) {
        // SAFETY: successful constructors gave owned references. No handle is
        // shared; the face is released before the blob, exactly once.
        unsafe {
            hb::hb_face_destroy(self.face);
            hb::hb_blob_destroy(self.blob);
        }
    }
}
fn main() {
    let path = std::env::args().nth(1).expect("font path argument");
    let bytes = std::fs::read(&path).expect("read pinned font");
    let length = u32::try_from(bytes.len()).expect("font length fits HarfBuzz");
    let ft_length = ft::FT_Long::try_from(bytes.len()).expect("font length fits FreeType");
    let mut library = ptr::null_mut();
    let mut face = ptr::null_mut();
    // SAFETY: out pointers address initialized local handle slots; immutable
    // bytes have checked extent and outlive every native face below. Successful
    // native results/nullness are checked before dereference. No pointers escape.
    unsafe {
        assert_eq!(ft::FT_Init_FreeType(&mut library), 0);
        assert!(!library.is_null());
        let error = ft::FT_New_Memory_Face(library, bytes.as_ptr(), ft_length, 0, &mut face);
        if error != 0 || face.is_null() {
            ft::FT_Done_FreeType(library);
            panic!("FT_New_Memory_Face failed: {error}");
        }
        let ft = FreeType { library, face };
        let mut major = 0;
        let mut minor = 0;
        let mut patch = 0;
        ft::FT_Library_Version(ft.library, &mut major, &mut minor, &mut patch);
        assert_eq!(
            (major, minor, patch),
            (2, 14, 3),
            "reject bundled/system FreeType fallback"
        );
        let ft_glyphs = u32::try_from((*ft.face).num_glyphs).expect("positive glyph count");
        let mut hb_major = 0;
        let mut hb_minor = 0;
        let mut hb_patch = 0;
        hb::hb_version(&mut hb_major, &mut hb_minor, &mut hb_patch);
        assert_eq!(
            (hb_major, hb_minor, hb_patch),
            (14, 5, 0),
            "reject system HarfBuzz fallback"
        );
        // DUPLICATE copies the checked byte slice; blob/face own their references.
        let blob = hb::hb_blob_create(
            bytes.as_ptr().cast(),
            length,
            hb::HB_MEMORY_MODE_DUPLICATE,
            ptr::null_mut(),
            None,
        );
        assert!(!blob.is_null());
        let hb_face = hb::hb_face_create(blob, 0);
        assert!(!hb_face.is_null());
        let hb = HarfBuzz {
            blob,
            face: hb_face,
        };
        let hb_glyphs = hb::hb_face_get_glyph_count(hb.face);
        assert!(hb_glyphs > 0);
        assert_eq!(ft_glyphs, hb_glyphs);
        println!(
            "NATIVE_SMOKE={{\"freetype\":\"{major}.{minor}.{patch}\",\"harfbuzz\":\"{hb_major}.{hb_minor}.{hb_patch}\",\"glyphs\":{hb_glyphs}}}"
        );
    }
    let bidi = unicode_bidi::BidiInfo::new("abc אבג", None);
    assert!(!bidi.levels[0].is_rtl());
    assert!(bidi.levels[4].is_rtl());
    assert_eq!('ع'.script(), Script::Arabic);
    assert_eq!('日'.script(), Script::Han);
    println!("UNICODE_SMOKE=Latin/Hebrew bidi, Arabic/Han script passed");
}
