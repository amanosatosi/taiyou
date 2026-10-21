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
pub(crate) fn is_mark(ch: char) -> Result<bool, FontError> {
    // SAFETY: HarfBuzz owns its immutable default Unicode funcs for process life.
    // No mutation, ownership transfer, callbacks or native pointer escape occurs.
    unsafe {
        let funcs = hb::hb_unicode_funcs_get_default();
        if funcs.is_null() {
            return Err(FontError::new(
                ErrorCode::NativeFailure,
                "missing Unicode funcs",
            ));
        }
        Ok(matches!(
            hb::hb_unicode_general_category(funcs, u32::from(ch)),
            hb::HB_UNICODE_GENERAL_CATEGORY_NON_SPACING_MARK
                | hb::HB_UNICODE_GENERAL_CATEGORY_SPACING_MARK
                | hb::HB_UNICODE_GENERAL_CATEGORY_ENCLOSING_MARK
        ))
    }
}
impl Drop for HbBuffer {
    fn drop(&mut self) {
        /* SAFETY: unique per-call buffer, copied results outlive it. */
        unsafe {
            hb::hb_buffer_destroy(self.0);
        }
    }
}

impl FontContext {
    pub(crate) fn decoration_metrics_authenticated(
        &mut self,
        blob: &FontBlob,
    ) -> Result<[(i16, i16); 2], FontError> {
        self.with_face(blob.shared_bytes(), blob.id().face_index(), |face| {
            // SAFETY: a validated worker-local face and a bounded fixed SFNT
            // buffer remain live throughout these synchronous reads.
            unsafe {
                let underline = (
                    (*face.raw).underline_position,
                    (*face.raw).underline_thickness,
                );
                let mut table = [0_u8; 30];
                let mut len = table.len() as ft::FT_ULong;
                if FT_Load_Sfnt_Table(
                    face.raw,
                    u32::from_be_bytes(*b"OS/2") as ft::FT_ULong,
                    0,
                    table.as_mut_ptr(),
                    &mut len,
                ) != 0
                    || len != 30
                {
                    return Err(FontError::new(
                        ErrorCode::FontUnavailable,
                        "strikeout OS/2 metrics unavailable",
                    ));
                }
                let strike = (
                    i16::from_be_bytes([table[28], table[29]]),
                    i16::from_be_bytes([table[26], table[27]]),
                );
                Ok([underline, strike])
            }
        })
    }
    /// Extract an owned, unhinted y-up vector path. This reauthenticates the blob,
    /// checks the collection face and glyph, and never publishes a native handle.
    pub fn outline(
        &mut self,
        blob: &FontBlob,
        glyph: u32,
        limits: &Limits,
    ) -> Result<GlyphOutline, FontError> {
        self.check_bytes(blob.bytes())?;
        crate::catalog::check_blob_identity(blob)?;
        self.outline_authenticated(blob, glyph, limits)
    }

    /// Only the catalog module may supply an immutable, authenticated blob.
    pub(crate) fn outline_authenticated(
        &mut self,
        blob: &FontBlob,
        glyph: u32,
        limits: &Limits,
    ) -> Result<GlyphOutline, FontError> {
        self.check_bytes(blob.bytes())?;
        let native_charge = (blob.bytes().len() as u64)
            .checked_mul(12)
            .and_then(|n| n.checked_add(64 * 1024))
            .ok_or_else(|| {
                FontError::new(
                    ErrorCode::LimitExceeded,
                    "outline native reservation overflow",
                )
            })?;
        if blob.bytes().len() as u64 > limits.font_bytes() {
            return Err(FontError::new(
                ErrorCode::LimitExceeded,
                "outline native reservation exceeded",
            ));
        }
        if native_charge > limits.transient_bytes() {
            return Err(FontError::transient_limit(
                "outline native reservation exceeded",
            ));
        }
        let native_limits = self.limits.clone();
        self.with_face(blob.shared_bytes(), blob.id().face_index(), |face| {
            // SAFETY: with_face retains library and authenticated bytes throughout
            // the call. FreeType validates its font storage; all native array counts,
            // pointers and contour endpoints are checked before decomposition.
            unsafe {
                if !u32::try_from((*face.raw).num_glyphs).is_ok_and(|n| glyph < n)
                    || (*face.raw).units_per_EM == 0
                {
                    return Err(FontError::new(
                        ErrorCode::InvalidArgument,
                        "outline glyph/metrics out of range",
                    ));
                }
                check_outline_face((*face.raw).face_flags)?;
                let flags = ft::FT_LOAD_NO_SCALE
                    | ft::FT_LOAD_NO_HINTING
                    | ft::FT_LOAD_NO_BITMAP
                    | ft::FT_LOAD_IGNORE_TRANSFORM;
                let code = ft::FT_Load_Glyph(face.raw, glyph, flags);
                if code != 0 {
                    return Err(native_error(code, "FT_Load_Glyph outline failed"));
                }
                let record = &*face.raw;
                let slot = record
                    .glyph
                    .as_ref()
                    .ok_or_else(|| native_error(0, "null glyph slot"))?;
                if slot.format != ft::FT_GLYPH_FORMAT_OUTLINE {
                    return Err(FontError::new(
                        ErrorCode::UnsupportedFeature,
                        "glyph has no vector outline",
                    ));
                }
                let outline = &slot.outline;
                let points = usize::try_from(outline.n_points)
                    .map_err(|_| native_error(0, "negative outline point count"))?;
                let contours = usize::try_from(outline.n_contours)
                    .map_err(|_| native_error(0, "negative contour count"))?;
                if (points == 0) != (contours == 0) || contours > points {
                    return Err(native_error(0, "inconsistent empty outline"));
                }
                // Core nonzero winding cannot silently reinterpret an even-odd outline.
                if outline.flags & 2 != 0 {
                    return Err(FontError::new(
                        ErrorCode::UnsupportedFeature,
                        "even-odd native glyph outline",
                    ));
                }
                let capacity = points
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(contours.checked_mul(2)?))
                    .ok_or_else(|| native_error(0, "outline count overflow"))?;
                for cap in [limits, &native_limits] {
                    let copy_limits = cap
                        .clone()
                        .with_transient_bytes(
                            cap.transient_bytes()
                                .checked_sub(native_charge)
                                .filter(|n| *n > 0)
                                .ok_or_else(|| {
                                    FontError::transient_limit("outline copy reservation exceeded")
                                })?,
                        )
                        .map_err(|e| {
                            FontError::new(e.code(), e.detail())
                                .with_resource_failure(e.resource_failure())
                        })?;
                    check_path_budget(capacity, &copy_limits).map_err(|e| {
                        FontError::new(e.code(), e.detail())
                            .with_resource_failure(e.resource_failure())
                    })?;
                }
                let mut state = OutlineCopy {
                    ops: Vec::new(),
                    failed: false,
                    open: false,
                };
                state.ops.try_reserve_exact(capacity).map_err(|_| {
                    FontError::new(
                        ErrorCode::AllocationFailed,
                        "outline copy allocation failed",
                    )
                })?;
                if points != 0 {
                    if outline.points.is_null()
                        || outline.tags.is_null()
                        || outline.contours.is_null()
                    {
                        return Err(native_error(0, "null outline arrays"));
                    }
                    let mut previous = -1;
                    for &end in std::slice::from_raw_parts(outline.contours, contours) {
                        if end <= previous || usize::try_from(end).map_or(true, |n| n >= points) {
                            return Err(native_error(0, "invalid contour endpoint"));
                        }
                        previous = end;
                    }
                    if usize::try_from(previous).ok() != points.checked_sub(1) {
                        return Err(native_error(0, "unconsumed outline points"));
                    }
                    for point in std::slice::from_raw_parts(outline.points, points) {
                        check_outline_point(point)?;
                    }
                    let code = FT_Outline_Check(outline);
                    if code != 0 {
                        return Err(native_error(code, "FT_Outline_Check failed"));
                    }
                    let funcs = outline_funcs();
                    let code = FT_Outline_Decompose(
                        outline,
                        &funcs,
                        (&mut state as *mut OutlineCopy).cast(),
                    );
                    if code != 0 || state.failed {
                        return Err(native_error(code, "FT_Outline_Decompose copy failed"));
                    }
                    state.close();
                    if state.failed {
                        return Err(native_error(0, "outline copy capacity exceeded"));
                    }
                }
                let owned = Outline::new(state.ops, limits).map_err(|e| {
                    FontError::new(e.code(), e.detail()).with_resource_failure(e.resource_failure())
                })?;
                let clockwise = FT_Outline_Get_Orientation(outline) == 0;
                GlyphOutline::new(blob.id(), glyph, record.units_per_EM, Arc::new(owned))
                    .map(|o| o.with_clockwise(clockwise))
            }
        })
    }
    #[cfg(test)]
    pub(crate) fn shape_raw(
        &mut self,
        blob: &FontBlob,
        request: &NativeRequest<'_>,
        limits: &Limits,
    ) -> Result<RawRun, FontError> {
        self.check_bytes(blob.bytes())?;
        crate::catalog::check_blob_identity(blob)?;
        self.shape_authenticated(blob, request, limits)
    }

    #[cfg(test)]
    pub(crate) fn shape_authenticated(
        &mut self,
        blob: &FontBlob,
        request: &NativeRequest<'_>,
        limits: &Limits,
    ) -> Result<RawRun, FontError> {
        self.shape_authenticated_with_ink(blob, request, limits, true, None)
    }

    pub(crate) fn shape_authenticated_with_ink(
        &mut self,
        blob: &FontBlob,
        request: &NativeRequest<'_>,
        limits: &Limits,
        collect_ink: bool,
        vertical: Option<crate::catalog::VerticalMetrics>,
    ) -> Result<RawRun, FontError> {
        self.check_bytes(blob.bytes())?;
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
        // The coordinator's FontContext cap is fixed before optional retention.
        // A failure of that cap cannot be helped by releasing retained layouts.
        if charge > self.limits.transient_bytes() {
            return Err(limited());
        }
        if charge > limits.transient_bytes() {
            return Err(limited().with_resource_failure(ResourceFailure::TransientMemory));
        }
        let font_len = u32::try_from(blob.bytes().len()).map_err(|_| limited())?;
        let bytes = blob.shared_bytes();
        self.preflight_native_work(context.len() as u64, request.range.len() as u64)?;
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
            let face_glyphs = hb::hb_face_get_glyph_count(face.raw);
            if scale <= 0 || face_glyphs == 0 {
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
                feature(u32::from_be_bytes(*b"vert"), u32::from(request.vertical)),
                feature(u32::from_be_bytes(*b"vkna"), u32::from(request.vertical)),
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
            self.record_native_glyphs(u64::from(count))?;
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
                if g.codepoint >= face_glyphs
                    || (cluster as usize) < request.range.start
                    || cluster as usize >= request.range.end
                    || !request.text.is_char_boundary(cluster as usize)
                {
                    return Err(failed());
                }
                let rotated = vertical.filter(|v| {
                    v.cff
                        || request.text[cluster as usize..]
                            .chars()
                            .next()
                            .is_some_and(|c| c >= '\u{2e80}')
                });
                let advance = if rotated.is_some() {
                    hb::hb_font_get_glyph_v_advance(font.raw, g.codepoint)
                        .checked_neg()
                        .ok_or_else(limited)?
                } else {
                    p.x_advance
                };
                let vertical_offset = if let Some(v) = rotated {
                    Some([
                        advance
                            .checked_add(i32::from(v.typo_descent))
                            .ok_or_else(limited)?,
                        i32::from(v.typo_descent),
                    ])
                } else {
                    None
                };
                glyphs.push(RawGlyph {
                    vertical_offset,
                    ink: {
                        let mut extents: hb::hb_glyph_extents_t = std::mem::zeroed();
                        if collect_ink
                            && hb::hb_font_get_glyph_extents(font.raw, g.codepoint, &mut extents)
                                != 0
                        {
                            Some([
                                extents.x_bearing,
                                extents.y_bearing,
                                extents.width,
                                extents.height,
                            ])
                        } else {
                            None
                        }
                    },
                    glyph: g.codepoint,
                    cluster,
                    advance_x: advance,
                    advance_y: p.y_advance,
                    offset_x: p.x_offset,
                    offset_y: p.y_offset,
                });
            }
            // Keep ownership explicit until after the last native read.
            let _ = font.face.raw;
            Ok(RawRun { glyphs, upem })
        }
    }
}

// Public FreeType outline callback ABI. Kept here so neither native pointers nor
// callback state cross the one reviewed unsafe boundary. C ABI checks accompany it.
#[repr(C)]
struct OutlineFuncs {
    move_to: extern "C" fn(*const ft::FT_Vector, *mut std::ffi::c_void) -> i32,
    line_to: extern "C" fn(*const ft::FT_Vector, *mut std::ffi::c_void) -> i32,
    conic_to:
        extern "C" fn(*const ft::FT_Vector, *const ft::FT_Vector, *mut std::ffi::c_void) -> i32,
    cubic_to: extern "C" fn(
        *const ft::FT_Vector,
        *const ft::FT_Vector,
        *const ft::FT_Vector,
        *mut std::ffi::c_void,
    ) -> i32,
    shift: ft::FT_Int,
    delta: ft::FT_Pos,
}
fn outline_funcs() -> OutlineFuncs {
    OutlineFuncs {
        move_to: copy_move,
        line_to: copy_line,
        conic_to: copy_quad,
        cubic_to: copy_cubic,
        // Retain exact half-design-unit implied TrueType conic endpoints.
        // FreeType scales before calculating midpoints; callbacks divide by 2.
        shift: 1,
        delta: 0,
    }
}
unsafe extern "C" {
    fn FT_Get_Font_Format(face: ft::FT_Face) -> *const std::ffi::c_char;
    fn FT_Outline_Get_Orientation(outline: *const ft::FT_Outline) -> ft::FT_Int;
    fn FT_Outline_Check(outline: *const ft::FT_Outline) -> ft::FT_Error;
    fn FT_Outline_Decompose(
        outline: *const ft::FT_Outline,
        funcs: *const OutlineFuncs,
        user: *mut std::ffi::c_void,
    ) -> ft::FT_Error;
}
struct OutlineCopy {
    ops: Vec<PathOp<FontPoint>>,
    failed: bool,
    open: bool,
}
fn check_outline_face(flags: ft::FT_Long) -> Result<(), FontError> {
    // FreeType documents that NO_SCALE can produce meaningless contours for
    // instruction-dependent tricky faces. Do not claim those are unhinted paths.
    if flags & ft::FT_FACE_FLAG_TRICKY != 0 {
        return Err(FontError::new(
            ErrorCode::UnsupportedFeature,
            "instruction-dependent tricky font requires a reviewed extraction policy",
        ));
    }
    Ok(())
}
impl OutlineCopy {
    fn push(&mut self, op: PathOp<FontPoint>) {
        // No allocation, indexing panic or unwinding across a C callback.
        if self.ops.len() == self.ops.capacity() {
            self.failed = true;
        } else {
            self.ops.push(op);
        }
    }
    fn close(&mut self) {
        if self.open {
            self.push(PathOp::Close);
            self.open = false;
        }
    }
}
fn copy_point(p: &ft::FT_Vector) -> Option<FontPoint> {
    FontPoint::new(
        f64::from(i32::try_from(i128::from(p.x)).ok()?) / 2.0,
        f64::from(i32::try_from(i128::from(p.y)).ok()?) / 2.0,
    )
    .ok()
}
fn check_outline_point(p: &ft::FT_Vector) -> Result<(), FontError> {
    // Bound scaled coordinates AND their sums before C computes conic midpoints.
    // This common bound keeps FT_Pos arithmetic valid on Windows' 32-bit long
    // as well as LP64 platforms, independent of the host's native word width.
    let cap = i128::from(i32::MAX) / 4;
    if [i128::from(p.x), i128::from(p.y)]
        .iter()
        .any(|v| v.abs() > cap)
    {
        return Err(FontError::new(
            ErrorCode::LimitExceeded,
            "native outline midpoint arithmetic overflow",
        ));
    }
    Ok(())
}
fn copy_callback(user: *mut std::ffi::c_void, pointers: &[*const ft::FT_Vector], kind: u8) -> i32 {
    // SAFETY: only FT_Outline_Decompose invokes these private callbacks with the
    // live exclusive OutlineCopy and checked native vectors. No references escape.
    unsafe {
        let Some(state) = user.cast::<OutlineCopy>().as_mut() else {
            return 1;
        };
        if state.failed {
            return 1;
        }
        let mut points = [FontPoint::ZERO; 3];
        for (index, &pointer) in pointers.iter().enumerate() {
            let Some(p) = pointer.as_ref().and_then(copy_point) else {
                state.failed = true;
                return 1;
            };
            let Some(slot) = points.get_mut(index) else {
                state.failed = true;
                return 1;
            };
            *slot = p;
        }
        if kind == 0 {
            state.close();
            state.open = true;
        }
        if !state.open {
            state.failed = true;
            return 1;
        }
        state.push(match kind {
            0 => PathOp::MoveTo(points[0]),
            1 => PathOp::LineTo(points[0]),
            2 => PathOp::QuadTo(points[0], points[1]),
            _ => PathOp::CubicTo(points[0], points[1], points[2]),
        });
        i32::from(state.failed)
    }
}
extern "C" fn copy_move(p: *const ft::FT_Vector, u: *mut std::ffi::c_void) -> i32 {
    copy_callback(u, &[p], 0)
}
extern "C" fn copy_line(p: *const ft::FT_Vector, u: *mut std::ffi::c_void) -> i32 {
    copy_callback(u, &[p], 1)
}
extern "C" fn copy_quad(
    a: *const ft::FT_Vector,
    b: *const ft::FT_Vector,
    u: *mut std::ffi::c_void,
) -> i32 {
    copy_callback(u, &[a, b], 2)
}
extern "C" fn copy_cubic(
    a: *const ft::FT_Vector,
    b: *const ft::FT_Vector,
    c: *const ft::FT_Vector,
    u: *mut std::ffi::c_void,
) -> i32 {
    copy_callback(u, &[a, b, c], 3)
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
    fn FT_Load_Sfnt_Table(
        face: ft::FT_Face,
        tag: ft::FT_ULong,
        offset: ft::FT_Long,
        buffer: *mut ft::FT_Byte,
        length: *mut ft::FT_ULong,
    ) -> ft::FT_Error;
}
fn native_error(code: ft::FT_Error, detail: &str) -> FontError {
    FontError::new(ErrorCode::NativeFailure, detail).with_native_code(i64::from(code))
}
#[cfg(test)]
std::thread_local! {
    static LIVE: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };
}
struct Library(ft::FT_Library);
/// Per-call-worker raster owner, neither Send nor Sync. No native handle escapes.
/// ```compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<taiyou_font::RasterContext>();
/// ```
/// ```compile_fail
/// fn require_sync<T: Sync>() {}
/// require_sync::<taiyou_font::RasterContext>();
/// ```
pub struct RasterContext {
    owner: FontContext,
}
/// Bounded numerical failure evidence, never contour/font bytes. This preserves
/// LimitExceeded and the comparator's error mapping while identifying its check.
#[derive(Clone, Copy, Debug, Default)]
struct StrokeLimitEvidence {
    check: &'static str,
    input_ops: usize,
    input_points: usize,
    input_contours: usize,
    output_points: u32,
    output_contours: u32,
    radius_q6: i32,
    input_bounds: Option<PixelRect>,
    output_bounds: Option<PixelRect>,
    exported_hull_q6: Option<(i128, i128, i128, i128)>,
    remaining_segments: u64,
    transient_available: u64,
    base_reservation: u64,
    export_reservation: u64,
}
impl StrokeLimitEvidence {
    fn detail(self) -> String {
        format!(
            "check={} input_ops={} input_points={} input_contours={} output_points={} output_contours={} radius_q6={} input_bounds={:?} output_bounds={:?} exported_hull_q6={:?} remaining_segments={} transient_available={} base_reservation={} export_reservation={}",
            self.check,
            self.input_ops,
            self.input_points,
            self.input_contours,
            self.output_points,
            self.output_contours,
            self.radius_q6,
            self.input_bounds,
            self.output_bounds,
            self.exported_hull_q6,
            self.remaining_segments,
            self.transient_available,
            self.base_reservation,
            self.export_reservation,
        )
    }
}
impl RasterContext {
    pub fn new() -> Result<Self, RenderError> {
        Self::with_limits(Limits::default())
    }
    pub fn with_limits(limits: Limits) -> Result<Self, RenderError> {
        FontContext::with_limits(limits)
            .map(|owner| Self { owner })
            .map_err(|e| {
                let mut r = RenderError::new(e.code(), e.detail())
                    .with_resource_failure(e.resource_failure());
                if let Some(code) = e.native_code() {
                    r = r.with_native_code(code);
                }
                r
            })
    }
    pub fn limits(&self) -> &Limits {
        &self.owner.limits
    }
    pub const fn backend(&self) -> &'static str {
        "FreeType 2.14.3 smooth/ftgrays; AA|DIRECT|CLIP=7; nonzero; CoreV1 raster policy 1"
    }
    /// Low-level checked absolute y-down Q6 contours. Bounds must contain their
    /// full control hull; stride permits initialized zero row padding.
    pub fn rasterize_path(
        &mut self,
        ops: &[PathOp<DevicePoint>],
        bounds: PixelRect,
        stride: usize,
    ) -> Result<Arc<Mask>, RenderError> {
        let invalid = || {
            RenderError::new(
                ErrorCode::InvalidArgument,
                "invalid raster path/bounds/stride",
            )
        };
        let limit = || {
            RenderError::new(
                ErrorCode::LimitExceeded,
                "native raster extent/count/reservation limit",
            )
        };
        let alloc = || {
            RenderError::new(
                ErrorCode::AllocationFailed,
                "raster buffer allocation failed",
            )
        };
        let limits = &self.owner.limits;
        check_path_budget(ops.len(), limits)?;
        let (width, height) = (
            usize::try_from(bounds.width()).map_err(|_| limit())?,
            usize::try_from(bounds.height()).map_err(|_| limit())?,
        );
        if bounds.is_empty() {
            if width != 0 || height != 0 || stride != 0 || !ops.is_empty() {
                return Err(invalid());
            }
            return Ok(Arc::new(Mask::new(bounds, 0, Arc::from([]))?));
        }
        if width > u16::MAX as usize
            || height > u16::MAX as usize
            || bounds.width() > limits.max_dimension()
            || bounds.height() > limits.max_dimension()
        {
            return Err(limit());
        }
        if stride < width {
            return Err(invalid());
        }
        let length = stride.checked_mul(height).ok_or_else(limit)?;
        let pixels = (width as u64)
            .checked_mul(height as u64)
            .ok_or_else(limit)?;
        let mut points_count = 0usize;
        let mut contours_count = 0usize;
        let mut open = false;
        for &op in ops {
            let n = match op {
                PathOp::MoveTo(_) if !open => {
                    open = true;
                    1
                }
                PathOp::LineTo(_) if open => 1,
                PathOp::QuadTo(..) if open => 2,
                PathOp::CubicTo(..) if open => 3,
                PathOp::Close if open => {
                    open = false;
                    contours_count = contours_count.checked_add(1).ok_or_else(limit)?;
                    0
                }
                _ => return Err(invalid()),
            };
            points_count = points_count.checked_add(n).ok_or_else(limit)?;
        }
        if open || points_count == 0 {
            return Err(invalid());
        }
        // Current reviewed binding exposes signed-short outline counts. Never
        // split a winding group to bypass its count limit.
        let native_points = i16::try_from(points_count).map_err(|_| limit())?;
        let native_contours = i16::try_from(contours_count).map_err(|_| limit())?;
        let charge = (length as u64)
            .checked_mul(3)
            .and_then(|n| {
                n.checked_add(
                    (ops.len() as u64)
                        .checked_mul(3 * std::mem::size_of::<PathOp<DevicePoint>>() as u64)?,
                )
            })
            .and_then(|n| n.checked_add((points_count as u64).checked_mul(256)?))
            .and_then(|n| n.checked_add(((width + height) as u64).checked_mul(64)?))
            .and_then(|n| n.checked_add(2 * 1024 * 1024))
            .ok_or_else(limit)?;
        if pixels > limits.output_pixels() || length as u64 > limits.output_bytes() {
            return Err(limit());
        }
        if charge > limits.transient_bytes() {
            return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
        }
        let mut points = Vec::new();
        let mut tags = Vec::new();
        let mut contours = Vec::new();
        points
            .try_reserve_exact(points_count)
            .map_err(|_| alloc())?;
        tags.try_reserve_exact(points_count).map_err(|_| alloc())?;
        contours
            .try_reserve_exact(contours_count)
            .map_err(|_| alloc())?;
        let convert = |p: DevicePoint| -> Result<ft::FT_Vector, RenderError> {
            let x = i64::from(p.x().ticks()) - i64::from(bounds.x0()) * 64;
            let y = i64::from(bounds.y1()) * 64 - i64::from(p.y().ticks());
            if x < 0 || y < 0 || x > width as i64 * 64 || y > height as i64 * 64 {
                return Err(invalid());
            }
            Ok(ft::FT_Vector {
                x: ft::FT_Pos::try_from(i128::from(x)).map_err(|_| limit())?,
                y: ft::FT_Pos::try_from(i128::from(y)).map_err(|_| limit())?,
            })
        };
        for &op in ops {
            let mut push = |p, tag| -> Result<(), RenderError> {
                points.push(convert(p)?);
                tags.push(tag);
                Ok(())
            };
            match op {
                PathOp::MoveTo(p) | PathOp::LineTo(p) => push(p, 1)?,
                PathOp::QuadTo(a, b) => {
                    push(a, 0)?;
                    push(b, 1)?;
                }
                PathOp::CubicTo(a, b, c) => {
                    push(a, 2)?;
                    push(b, 2)?;
                    push(c, 1)?;
                }
                PathOp::Close => {
                    contours.push(i16::try_from(points.len() - 1).map_err(|_| limit())?)
                }
            }
        }
        let mut state = SpanCopy {
            data: Vec::new(),
            width,
            height,
            stride,
            failed: false,
            delivered: 0,
        };
        state.data.try_reserve_exact(length).map_err(|_| alloc())?;
        state.data.resize(length, 0);
        let outline = ft::FT_Outline {
            n_contours: native_contours,
            n_points: native_points,
            points: points.as_mut_ptr(),
            tags: tags.as_mut_ptr(),
            contours: contours.as_mut_ptr(),
            flags: 0,
        };
        let mut params = RasterParams {
            target: ptr::null(),
            source: ptr::null(),
            flags: 7,
            gray_spans: Some(copy_spans),
            black_spans: None,
            bit_test: None,
            bit_set: None,
            user: (&mut state as *mut SpanCopy).cast(),
            clip_box: ft::FT_BBox {
                xMin: 0,
                yMin: 0,
                xMax: ft::FT_Pos::from(width as u16),
                yMax: ft::FT_Pos::from(height as u16),
            },
        };
        // SAFETY: initialized library, fully owned checked arrays and synchronous
        // callback state stay live until return. No pointers are published. Flags
        // select only smooth grayscale nonzero coverage, clipped to its full hull.
        let code = unsafe { FT_Outline_Render(self.owner.library.0, &outline, &mut params) };
        if code != 0 {
            return Err(
                RenderError::new(ErrorCode::NativeFailure, "FT_Outline_Render failed")
                    .with_native_code(i64::from(code)),
            );
        }
        if state.failed {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "invalid native coverage spans",
            ));
        }
        if state.data.iter().all(|&v| v == 0) {
            return Ok(Arc::new(Mask::new(
                PixelRect::new(0, 0, 0, 0).map_err(|_| invalid())?,
                0,
                Arc::from([]),
            )?));
        }
        Ok(Arc::new(Mask::new(bounds, stride, state.data.into())?))
    }

    /// Rasterize the outside contour of a round isotropic FreeType stroke.
    /// Input contours are transformed device-space Q6 geometry; the returned
    /// A8 mask is the expanded silhouette E, including full support outside the
    /// original path and negative device origins.
    pub fn stroke_rasterize_path(
        &mut self,
        ops: &[PathOp<DevicePoint>],
        radius: Q6,
        remaining_segments: u64,
    ) -> Result<(Arc<Mask>, u64), RenderError> {
        self.stroke_rasterize_path_axes(ops, radius, radius, remaining_segments)
    }
    /// Elliptical round joins: normalize the device-space axes before the
    /// scalar round stroker, then restore them before coverage rasterization.
    pub fn stroke_rasterize_path_axes(
        &mut self,
        ops: &[PathOp<DevicePoint>],
        radius_x: Q6,
        radius_y: Q6,
        remaining_segments: u64,
    ) -> Result<(Arc<Mask>, u64), RenderError> {
        self.stroke_rasterize_path_axes_impl(ops,radius_x,radius_y,remaining_segments,None)
    }
    /// Keep the validated round-stroker geometry while supplying a separate
    /// coverage rasterizer. Only owned path values cross the native boundary.
    pub fn stroke_rasterize_path_axes_with_coverage(
        &mut self,ops:&[PathOp<DevicePoint>],radius_x:Q6,radius_y:Q6,remaining_segments:u64,
        coverage:fn(&[PathOp<DevicePoint>],&Limits)->Result<Arc<Mask>,RenderError>,
    )->Result<(Arc<Mask>,u64),RenderError>{
        self.stroke_rasterize_path_axes_impl(ops,radius_x,radius_y,remaining_segments,Some(coverage))
    }
    #[allow(clippy::type_complexity)]
    fn stroke_rasterize_path_axes_impl(
        &mut self,ops:&[PathOp<DevicePoint>],radius_x:Q6,radius_y:Q6,remaining_segments:u64,
        coverage:Option<fn(&[PathOp<DevicePoint>],&Limits)->Result<Arc<Mask>,RenderError>>,
    )->Result<(Arc<Mask>,u64),RenderError>{
        let radius = radius_x.max(radius_y);
        let invalid =
            || RenderError::new(ErrorCode::InvalidArgument, "invalid stroke path or radius");
        let evidence = std::cell::Cell::new(StrokeLimitEvidence {
            check: "input-counts",
            input_ops: ops.len(),
            radius_q6: radius.ticks(),
            remaining_segments,
            transient_available: self.owner.limits.transient_bytes(),
            ..StrokeLimitEvidence::default()
        });
        let limit = || {
            RenderError::new(
                ErrorCode::LimitExceeded,
                &format!(
                    "native stroke extent/count/reservation limit: {}",
                    evidence.get().detail()
                ),
            )
        };
        let alloc = || {
            RenderError::new(
                ErrorCode::AllocationFailed,
                "stroke buffer allocation failed",
            )
        };
        let limits = &self.owner.limits;
        check_path_budget(ops.len(), limits)?;
        if radius_x.ticks() < 0 || radius_y.ticks() < 0 {
            return Err(invalid());
        }
        if ops.is_empty() {
            return Ok((
                Arc::new(Mask::new(
                    PixelRect::new(0, 0, 0, 0).map_err(|_| invalid())?,
                    0,
                    Arc::from([]),
                )?),
                0,
            ));
        }

        let mut point_count = 0usize;
        let mut contour_count = 0usize;
        let mut open = false;
        let mut hull: Option<(i64, i64, i64, i64)> = None;
        let mut observe = |p: DevicePoint| {
            let x = i64::from(p.x().ticks());
            let y = i64::from(p.y().ticks());
            hull = Some(hull.map_or((x, y, x, y), |(x0, y0, x1, y1)| {
                (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
            }));
        };
        for &op in ops {
            match op {
                PathOp::MoveTo(p) if !open => {
                    open = true;
                    point_count = point_count.checked_add(1).ok_or_else(limit)?;
                    observe(p);
                }
                PathOp::LineTo(p) if open => {
                    point_count = point_count.checked_add(1).ok_or_else(limit)?;
                    observe(p);
                }
                PathOp::QuadTo(a, b) if open => {
                    point_count = point_count.checked_add(2).ok_or_else(limit)?;
                    observe(a);
                    observe(b);
                }
                PathOp::CubicTo(a, b, c) if open => {
                    point_count = point_count.checked_add(3).ok_or_else(limit)?;
                    observe(a);
                    observe(b);
                    observe(c);
                }
                PathOp::Close if open => {
                    open = false;
                    contour_count = contour_count.checked_add(1).ok_or_else(limit)?;
                }
                _ => return Err(invalid()),
            }
        }
        if open || point_count == 0 || contour_count == 0 {
            return Err(invalid());
        }
        evidence.set(StrokeLimitEvidence {
            input_points: point_count,
            input_contours: contour_count,
            ..evidence.get()
        });
        let native_points = i16::try_from(point_count).map_err(|_| limit())?;
        let native_contours = i16::try_from(contour_count).map_err(|_| limit())?;
        let (min_x, min_y, max_x, max_y) = hull.ok_or_else(invalid)?;
        let radius = i64::from(radius.ticks());
        let floor_pixel = |ticks: i64| i32::try_from(ticks.div_euclid(64)).map_err(|_| limit());
        let ceil_pixel = |ticks: i64| i32::try_from(-(-ticks).div_euclid(64)).map_err(|_| limit());
        let input_x0 = floor_pixel(min_x)?;
        let input_y0 = floor_pixel(min_y)?;
        let mut input_x1 = ceil_pixel(max_x)?;
        let mut input_y1 = ceil_pixel(max_y)?;
        if input_x0 == input_x1 {
            input_x1 = input_x0.checked_add(1).ok_or_else(limit)?;
        }
        if input_y0 == input_y1 {
            input_y1 = input_y0.checked_add(1).ok_or_else(limit)?;
        }
        let input_bounds =
            PixelRect::new(input_x0, input_y0, input_x1, input_y1).map_err(|_| invalid())?;
        if radius == 0 {
            if let Some(coverage)=coverage {return coverage(ops,limits).map(|mask|(mask,0));}
            let stride = usize::try_from(input_bounds.width()).map_err(|_| limit())?;
            return self
                .rasterize_path(ops, input_bounds, stride)
                .map(|mask| (mask, 0));
        }
        if radius_x.ticks() == 0 || radius_y.ticks() == 0 {
            return Err(RenderError::new(
                ErrorCode::UnsupportedFeature,
                "degenerate single-axis outline widening",
            ));
        }
        let mut bounds = PixelRect::new(
            floor_pixel(min_x.checked_sub(radius).ok_or_else(limit)?)?,
            floor_pixel(min_y.checked_sub(radius).ok_or_else(limit)?)?,
            ceil_pixel(max_x.checked_add(radius).ok_or_else(limit)?)?,
            ceil_pixel(max_y.checked_add(radius).ok_or_else(limit)?)?,
        )
        .map_err(|_| limit())?;
        let (mut width, mut height) = (
            usize::try_from(bounds.width()).map_err(|_| limit())?,
            usize::try_from(bounds.height()).map_err(|_| limit())?,
        );
        evidence.set(StrokeLimitEvidence {
            check: "mask-extent",
            input_bounds: Some(input_bounds),
            output_bounds: Some(bounds),
            ..evidence.get()
        });
        let input_width = i64::from(input_bounds.width());
        let input_height = i64::from(input_bounds.height());
        let mut output_width = i64::try_from(width).map_err(|_| limit())?;
        let mut output_height = i64::try_from(height).map_err(|_| limit())?;
        if width > u16::MAX as usize
            || height > u16::MAX as usize
            || bounds.width() > limits.max_dimension()
            || bounds.height() > limits.max_dimension()
            || input_bounds.width() > limits.max_dimension()
            || input_bounds.height() > limits.max_dimension()
            || radius > i64::from(limits.max_dimension()) * 64
        {
            return Err(limit());
        }
        let mut length = width.checked_mul(height).ok_or_else(limit)?;
        let pixels = u64::try_from(length).map_err(|_| limit())?;
        evidence.set(StrokeLimitEvidence {
            check: "mask-pixels",
            ..evidence.get()
        });
        if pixels > limits.output_pixels() || pixels > limits.output_bytes() {
            return Err(limit());
        }
        let mut contours = Vec::new();
        let mut points = Vec::new();
        let mut tags = Vec::new();
        let input_length = usize::try_from(input_width)
            .ok()
            .and_then(|w| {
                usize::try_from(input_height)
                    .ok()
                    .and_then(|h| w.checked_mul(h))
            })
            .ok_or_else(limit)?;
        let base_charge = (length as u64)
            .checked_mul(3)
            .and_then(|n| {
                n.checked_add(
                    (ops.len() as u64)
                        .checked_mul(3 * std::mem::size_of::<PathOp<DevicePoint>>() as u64)?,
                )
            })
            .and_then(|n| n.checked_add((point_count as u64).checked_mul(256)?))
            .and_then(|n| n.checked_add(((width + height) as u64).checked_mul(64)?))
            .and_then(|n| n.checked_add((input_length as u64).checked_mul(3)?))
            .and_then(|n| n.checked_add(2 * 1024 * 1024))
            .ok_or_else(limit)?;
        evidence.set(StrokeLimitEvidence {
            check: "base-reservation",
            base_reservation: base_charge,
            ..evidence.get()
        });
        if base_charge > limits.transient_bytes() {
            return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
        }
        contours
            .try_reserve_exact(contour_count)
            .map_err(|_| alloc())?;
        points.try_reserve_exact(point_count).map_err(|_| alloc())?;
        tags.try_reserve_exact(point_count).map_err(|_| alloc())?;
        let convert = |p: DevicePoint| -> Result<ft::FT_Vector, RenderError> {
            let x = i64::from(p.x().ticks()) - i64::from(input_bounds.x0()) * 64;
            let y = i64::from(input_bounds.y1()) * 64 - i64::from(p.y().ticks());
            if x < 0 || y < 0 || x > input_width * 64 || y > input_height * 64 {
                return Err(invalid());
            }
            Ok(ft::FT_Vector {
                x: ft::FT_Pos::try_from(i128::from(x)).map_err(|_| limit())?,
                y: ft::FT_Pos::try_from(i128::from(y)).map_err(|_| limit())?,
            })
        };
        for &op in ops {
            let mut push = |p, tag| -> Result<(), RenderError> {
                points.push(convert(p)?);
                tags.push(tag);
                Ok(())
            };
            match op {
                PathOp::MoveTo(p) | PathOp::LineTo(p) => push(p, 1)?,
                PathOp::QuadTo(a, b) => {
                    push(a, 0)?;
                    push(b, 1)?;
                }
                PathOp::CubicTo(a, b, c) => {
                    push(a, 2)?;
                    push(b, 2)?;
                    push(c, 1)?;
                }
                PathOp::Close => {
                    contours.push(i16::try_from(points.len() - 1).map_err(|_| limit())?)
                }
            }
        }
        let axes = (i128::from(radius_x.ticks()), i128::from(radius_y.ticks()));
        if radius_x != radius_y {
            for p in &mut points {
                p.x = ft::FT_Pos::try_from(i128::from(p.x) * i128::from(radius) / axes.0)
                    .map_err(|_| limit())?;
                p.y = ft::FT_Pos::try_from(i128::from(p.y) * i128::from(radius) / axes.1)
                    .map_err(|_| limit())?;
            }
        }
        let mut outline = ft::FT_Outline {
            n_contours: native_contours,
            n_points: native_points,
            points: points.as_mut_ptr(),
            tags: tags.as_mut_ptr(),
            contours: contours.as_mut_ptr(),
            flags: 0,
        };
        // SAFETY: the outline arrays are fully initialized, sized by checked
        // signed-short counts, and remain exclusively borrowed through each call.
        let code = unsafe { FT_Outline_Check(&outline) };
        if code != 0 {
            return Err(
                RenderError::new(ErrorCode::NativeFailure, "FT_Outline_Check failed")
                    .with_native_code(i64::from(code)),
            );
        }
        let mut raw_stroker: *mut std::ffi::c_void = ptr::null_mut();
        // SAFETY: the worker-local initialized library is live and the output
        // pointer is a valid exclusive slot for one opaque stroker handle.
        let code = unsafe { FT_Stroker_New(self.owner.library.0, &mut raw_stroker) };
        if code != 0 {
            if !raw_stroker.is_null() {
                // SAFETY: a non-null handle returned alongside the error is still
                // owned here and must be released before returning.
                unsafe { FT_Stroker_Done(raw_stroker) };
            }
            return Err(
                RenderError::new(ErrorCode::NativeFailure, "FT_Stroker_New failed")
                    .with_native_code(i64::from(code)),
            );
        }
        if raw_stroker.is_null() {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "FT_Stroker_New returned a null handle",
            ));
        }
        let stroker = Stroker(raw_stroker);
        // FreeType documents radius in outline units. Round line caps and joins
        // are explicit; closed glyph contours do not expose the cap setting.
        let native_radius = ft::FT_Fixed::try_from(radius).map_err(|_| limit())?;
        // SAFETY: the checked radius fits FT_Fixed; enum values are the stable
        // public FreeType round-cap (1) and round-join (0) constants.
        unsafe { FT_Stroker_Set(stroker.0, native_radius, 1, 0, 65_536) };
        // SAFETY: stroker and checked outline stay live for the synchronous call;
        // opened=0 preserves each source contour as a closed filled boundary.
        let code = unsafe { FT_Stroker_ParseOutline(stroker.0, &mut outline, 0) };
        if code != 0 {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "FT_Stroker_ParseOutline failed",
            )
            .with_native_code(i64::from(code)));
        }
        // SAFETY: FreeType inspects the still-live validated outline and returns
        // the side that bounds its nonzero-winding filled region.
        let outside = unsafe { FT_Outline_GetOutsideBorder(&mut outline) };
        let mut output_points = 0_u32;
        let mut output_contours = 0_u32;
        // SAFETY: output count slots are valid and stroker owns its parsed paths.
        let code = unsafe {
            FT_Stroker_GetBorderCounts(stroker.0, outside, &mut output_points, &mut output_contours)
        };
        if code != 0 {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "FT_Stroker_GetBorderCounts failed",
            )
            .with_native_code(i64::from(code)));
        }
        let output_point_count = usize::try_from(output_points).map_err(|_| limit())?;
        let output_contour_count = usize::try_from(output_contours).map_err(|_| limit())?;
        evidence.set(StrokeLimitEvidence {
            check: "stroker-native-counts",
            output_points,
            output_contours,
            ..evidence.get()
        });
        let output_native_points = i16::try_from(output_points).map_err(|_| limit())?;
        let output_native_contours = i16::try_from(output_contours).map_err(|_| limit())?;
        evidence.set(StrokeLimitEvidence {
            check: "frame-segments",
            ..evidence.get()
        });
        if u64::try_from(output_point_count).map_err(|_| limit())? > remaining_segments {
            return Err(limit());
        }
        let export_charge = (output_point_count as u64)
            .checked_mul(
                3 * (std::mem::size_of::<ft::FT_Vector>()
                    + std::mem::size_of::<u8>()
                    + std::mem::size_of::<PathOp<DevicePoint>>()) as u64,
            )
            .and_then(|n| {
                n.checked_add(
                    (output_contour_count as u64)
                        .checked_mul(3 * std::mem::size_of::<i16>() as u64)?,
                )
            })
            .and_then(|n| n.checked_add(base_charge))
            .ok_or_else(limit)?;
        evidence.set(StrokeLimitEvidence {
            check: "export-reservation",
            export_reservation: export_charge,
            ..evidence.get()
        });
        if export_charge > limits.transient_bytes() {
            return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
        }
        let mut expanded_points = Vec::new();
        let mut expanded_tags = Vec::new();
        let mut expanded_contours = Vec::new();
        expanded_points
            .try_reserve_exact(output_point_count)
            .map_err(|_| alloc())?;
        expanded_tags
            .try_reserve_exact(output_point_count)
            .map_err(|_| alloc())?;
        expanded_contours
            .try_reserve_exact(output_contour_count)
            .map_err(|_| alloc())?;
        expanded_points.resize(
            output_point_count,
            ft::FT_Vector {
                x: 0 as ft::FT_Pos,
                y: 0 as ft::FT_Pos,
            },
        );
        expanded_tags.resize(output_point_count, 0);
        expanded_contours.resize(output_contour_count, 0_i16);
        let mut expanded = ft::FT_Outline {
            n_contours: 0,
            n_points: 0,
            points: expanded_points.as_mut_ptr(),
            tags: expanded_tags.as_mut_ptr(),
            contours: expanded_contours.as_mut_ptr(),
            flags: 0,
        };
        // SAFETY: capacities exactly satisfy the previously queried counts;
        // FreeType appends only to these private initialized arrays.
        unsafe { FT_Stroker_ExportBorder(stroker.0, outside, &mut expanded) };
        if expanded.n_points != output_native_points
            || expanded.n_contours != output_native_contours
        {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "FT_Stroker_ExportBorder count mismatch",
            ));
        }
        if radius_x != radius_y {
            for p in &mut expanded_points {
                p.x = ft::FT_Pos::try_from(i128::from(p.x) * axes.0 / i128::from(radius))
                    .map_err(|_| limit())?;
                p.y = ft::FT_Pos::try_from(i128::from(p.y) * axes.1 / i128::from(radius))
                    .map_err(|_| limit())?;
            }
        }
        let x_shift = (i64::from(input_bounds.x0()) - i64::from(bounds.x0())) * 64;
        let y_shift = (i64::from(bounds.y1()) - i64::from(input_bounds.y1())) * 64;
        let exported_hull = expanded_points.iter().fold(None, |hull, p| {
            let x = i128::from(p.x) + i128::from(x_shift);
            let y = i128::from(p.y) + i128::from(y_shift);
            Some(hull.map_or(
                (x, y, x, y),
                |(x0, y0, x1, y1): (i128, i128, i128, i128)| {
                    (x0.min(x), y0.min(y), x1.max(x), y1.max(y))
                },
            ))
        });
        evidence.set(StrokeLimitEvidence {
            check: "export-point-bounds",
            exported_hull_q6: exported_hull,
            ..evidence.get()
        });
        // A round join's Bezier control points can exceed the nominal radius
        // box even when the curve itself remains close to that radius. The
        // predicted box is not a resource limit. Preserve the complete exported
        // control hull, then recheck the actual allocation before rasterizing.
        if let Some((x0, y0, x1, y1)) = exported_hull {
            let x_origin = i128::from(bounds.x0()) * 64;
            let y_origin = i128::from(bounds.y1()) * 64;
            let ticks = |n| i64::try_from(n).map_err(|_| limit());
            let actual = PixelRect::new(
                bounds.x0().min(floor_pixel(ticks(x_origin + x0)?)?),
                bounds.y0().min(floor_pixel(ticks(y_origin - y1)?)?),
                bounds.x1().max(ceil_pixel(ticks(x_origin + x1)?)?),
                bounds.y1().max(ceil_pixel(ticks(y_origin - y0)?)?),
            )
            .map_err(|_| limit())?;
            evidence.set(StrokeLimitEvidence {
                check: "exported-mask-extent",
                output_bounds: Some(actual),
                ..evidence.get()
            });
            if actual.width() > limits.max_dimension()
                || actual.height() > limits.max_dimension()
                || actual.width() > u32::from(u16::MAX)
                || actual.height() > u32::from(u16::MAX)
            {
                return Err(limit());
            }
            let actual_width = usize::try_from(actual.width()).map_err(|_| limit())?;
            let actual_height = usize::try_from(actual.height()).map_err(|_| limit())?;
            let actual_length = actual_width.checked_mul(actual_height).ok_or_else(limit)?;
            let actual_pixels = u64::try_from(actual_length).map_err(|_| limit())?;
            if actual_pixels > limits.output_pixels() || actual_pixels > limits.output_bytes() {
                return Err(limit());
            }
            let added_charge = u64::try_from(actual_length - length)
                .ok()
                .and_then(|n| n.checked_mul(3))
                .and_then(|n| {
                    n.checked_add(
                        u64::try_from(actual_width + actual_height - width - height)
                            .ok()?
                            .checked_mul(64)?,
                    )
                })
                .ok_or_else(limit)?;
            let actual_base = base_charge.checked_add(added_charge).ok_or_else(limit)?;
            let actual_export = export_charge.checked_add(added_charge).ok_or_else(limit)?;
            evidence.set(StrokeLimitEvidence {
                check: "exported-mask-reservation",
                base_reservation: actual_base,
                export_reservation: actual_export,
                ..evidence.get()
            });
            if actual_export > limits.transient_bytes() {
                return Err(limit().with_resource_failure(ResourceFailure::TransientMemory));
            }
            bounds = actual;
            width = actual_width;
            height = actual_height;
            length = actual_length;
            output_width = i64::try_from(width).map_err(|_| limit())?;
            output_height = i64::try_from(height).map_err(|_| limit())?;
        }
        let x_shift = (i64::from(input_bounds.x0()) - i64::from(bounds.x0())) * 64;
        let y_shift = (i64::from(bounds.y1()) - i64::from(input_bounds.y1())) * 64;
        evidence.set(StrokeLimitEvidence {
            check: "export-point-bounds",
            ..evidence.get()
        });
        for point in &mut expanded_points {
            let x = i128::from(point.x) + i128::from(x_shift);
            let y = i128::from(point.y) + i128::from(y_shift);
            point.x = ft::FT_Pos::try_from(x).map_err(|_| limit())?;
            point.y = ft::FT_Pos::try_from(y).map_err(|_| limit())?;
            if i128::from(point.x) < 0
                || i128::from(point.y) < 0
                || i128::from(point.x) > i128::from(output_width) * 64
                || i128::from(point.y) > i128::from(output_height) * 64
            {
                return Err(limit());
            }
        }
        // SAFETY: the exported arrays remain exclusively owned, with count
        // fields checked against the allocated lengths and legal contour indices.
        let code = unsafe { FT_Outline_Check(&expanded) };
        if code != 0 {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "expanded FT_Outline_Check failed",
            )
            .with_native_code(i64::from(code)));
        }
        if let Some(coverage)=coverage {
            let capacity=output_point_count.checked_mul(2).and_then(|n|n.checked_add(output_contour_count*2+1)).ok_or_else(limit)?;
            let copy_charge=(capacity as u64).checked_mul(3*(std::mem::size_of::<PathOp<FontPoint>>()+std::mem::size_of::<PathOp<DevicePoint>>()) as u64).ok_or_else(limit)?;
            let live=evidence.get().export_reservation.checked_sub((length as u64)*3).and_then(|n|n.checked_add(copy_charge)).ok_or_else(limit)?;
            let available=limits.transient_bytes().checked_sub(live).filter(|&n|n>0).ok_or_else(||limit().with_resource_failure(ResourceFailure::TransientMemory))?;
            let mut state=OutlineCopy {ops:Vec::new(),failed:false,open:false};
            state.ops.try_reserve_exact(capacity).map_err(|_|alloc())?;
            let funcs=outline_funcs();
            // SAFETY: checked exported arrays and initialized callback storage
            // remain exclusively owned throughout synchronous decomposition.
            let code=unsafe {FT_Outline_Decompose(&expanded,&funcs,(&mut state as *mut OutlineCopy).cast())};
            state.close();
            if code!=0 || state.failed {return Err(RenderError::new(ErrorCode::NativeFailure,"stroked outline copy failed").with_native_code(i64::from(code)));}
            let mut path=Vec::new();path.try_reserve_exact(state.ops.len()).map_err(|_|alloc())?;
            for op in state.ops {
                path.push(op.try_map(|p|{
                    let x=f64::from(bounds.x0())+p.x().get()/64.0;
                    let y=f64::from(bounds.y1())-p.y().get()/64.0;
                    let q=|v|FiniteF64::new(v).and_then(Q6::from_pixels).map_err(|_|limit());
                    Ok(DevicePoint::new(q(x)?,q(y)?))
                })?);
            }
            let mask=coverage(&path,&limits.clone().with_transient_bytes(available).map_err(|_|limit())?)?;
            return Ok((mask,output_point_count as u64));
        }
        let mut state = SpanCopy {
            data: Vec::new(),
            width,
            height,
            stride: width,
            failed: false,
            delivered: 0,
        };
        state.data.try_reserve_exact(length).map_err(|_| alloc())?;
        state.data.resize(length, 0);
        let mut params = RasterParams {
            target: ptr::null(),
            source: ptr::null(),
            flags: 7,
            gray_spans: Some(copy_spans),
            black_spans: None,
            bit_test: None,
            bit_set: None,
            user: (&mut state as *mut SpanCopy).cast(),
            clip_box: ft::FT_BBox {
                xMin: 0,
                yMin: 0,
                xMax: ft::FT_Pos::try_from(width).map_err(|_| limit())?,
                yMax: ft::FT_Pos::try_from(height).map_err(|_| limit())?,
            },
        };
        // SAFETY: library, stroker, exported outline arrays, coverage buffer,
        // callback state and raster parameters all outlive the synchronous call.
        let code = unsafe { FT_Outline_Render(self.owner.library.0, &expanded, &mut params) };
        if code != 0 {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "stroked FT_Outline_Render failed",
            )
            .with_native_code(i64::from(code)));
        }
        if state.failed {
            return Err(RenderError::new(
                ErrorCode::NativeFailure,
                "invalid stroked native coverage spans",
            ));
        }
        if state.data.iter().all(|&v| v == 0) {
            return Ok((
                Arc::new(Mask::new(
                    PixelRect::new(0, 0, 0, 0).map_err(|_| invalid())?,
                    0,
                    Arc::from([]),
                )?),
                u64::try_from(output_point_count).map_err(|_| limit())?,
            ));
        }
        Ok((
            Arc::new(Mask::new(bounds, width, state.data.into())?),
            u64::try_from(output_point_count).map_err(|_| limit())?,
        ))
    }
}
#[repr(C)]
struct RasterSpan {
    x: u16,
    len: u16,
    coverage: u8,
}
type SpanFunc = extern "C" fn(i32, i32, *const RasterSpan, *mut std::ffi::c_void);
#[repr(C)]
struct RasterParams {
    target: *const std::ffi::c_void,
    source: *const std::ffi::c_void,
    flags: i32,
    gray_spans: Option<SpanFunc>,
    black_spans: Option<SpanFunc>,
    bit_test: Option<extern "C" fn(i32, i32, *mut std::ffi::c_void) -> i32>,
    bit_set: Option<extern "C" fn(i32, i32, *mut std::ffi::c_void)>,
    user: *mut std::ffi::c_void,
    clip_box: ft::FT_BBox,
}
unsafe extern "C" {
    fn FT_Outline_Render(
        library: ft::FT_Library,
        outline: *const ft::FT_Outline,
        params: *mut RasterParams,
    ) -> ft::FT_Error;
    fn FT_Stroker_New(library: ft::FT_Library, stroker: *mut *mut std::ffi::c_void)
    -> ft::FT_Error;
    fn FT_Stroker_Done(stroker: *mut std::ffi::c_void);
    fn FT_Stroker_Set(
        stroker: *mut std::ffi::c_void,
        radius: ft::FT_Fixed,
        line_cap: ft::FT_Int,
        line_join: ft::FT_Int,
        miter_limit: ft::FT_Fixed,
    );
    fn FT_Stroker_ParseOutline(
        stroker: *mut std::ffi::c_void,
        outline: *mut ft::FT_Outline,
        opened: ft::FT_Bool,
    ) -> ft::FT_Error;
    fn FT_Outline_GetOutsideBorder(outline: *mut ft::FT_Outline) -> ft::FT_Int;
    fn FT_Stroker_GetBorderCounts(
        stroker: *mut std::ffi::c_void,
        border: ft::FT_Int,
        points: *mut ft::FT_UInt,
        contours: *mut ft::FT_UInt,
    ) -> ft::FT_Error;
    fn FT_Stroker_ExportBorder(
        stroker: *mut std::ffi::c_void,
        border: ft::FT_Int,
        outline: *mut ft::FT_Outline,
    );
}
struct Stroker(*mut std::ffi::c_void);
impl Drop for Stroker {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: this wrapper uniquely owns the opaque handle returned by
            // FT_Stroker_New; every exit path releases it exactly once.
            unsafe { FT_Stroker_Done(self.0) };
        }
    }
}
struct SpanCopy {
    data: Vec<u8>,
    width: usize,
    height: usize,
    stride: usize,
    failed: bool,
    delivered: usize,
}
extern "C" fn copy_spans(
    y: i32,
    count: i32,
    spans: *const RasterSpan,
    user: *mut std::ffi::c_void,
) {
    // SAFETY: only the synchronous FreeType render call supplies a live exclusive
    // SpanCopy. Native output extents are checked before slicing or writing.
    unsafe {
        let Some(s) = user.cast::<SpanCopy>().as_mut() else {
            return;
        };
        if s.failed {
            return;
        }
        let (Ok(y), Ok(count)) = (usize::try_from(y), usize::try_from(count)) else {
            s.failed = true;
            return;
        };
        // The pinned ftgrays.c emits batches of at most 16 spans, including a
        // legitimate zero-length tail at a clipped band's right edge. Width is
        // therefore not a valid upper bound on the native batch count.
        if y >= s.height || count > 16 || count > 0 && spans.is_null() {
            s.failed = true;
            return;
        }
        if count == 0 {
            return;
        }
        let Some(delivered) = s.delivered.checked_add(count) else {
            s.failed = true;
            return;
        };
        if delivered > s.width.saturating_mul(s.height).saturating_mul(2) {
            s.failed = true;
            return;
        }
        s.delivered = delivered;
        let mut previous = 0usize;
        for span in std::slice::from_raw_parts(spans, count) {
            let x = usize::from(span.x);
            let n = usize::from(span.len);
            let Some(end) = x.checked_add(n) else {
                s.failed = true;
                return;
            };
            if x < previous || end > s.width {
                s.failed = true;
                return;
            }
            previous = end;
            if n == 0 {
                continue;
            }
            let Some(start) = (s.height - 1 - y)
                .checked_mul(s.stride)
                .and_then(|v| v.checked_add(x))
            else {
                s.failed = true;
                return;
            };
            let Some(end) = start.checked_add(n) else {
                s.failed = true;
                return;
            };
            let Some(dst) = s.data.get_mut(start..end) else {
                s.failed = true;
                return;
            };
            if dst.iter().any(|&v| v != 0) {
                s.failed = true;
                return;
            }
            dst.fill(span.coverage);
        }
    }
}
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
    pub aliases: Vec<String>,
    pub weight: u16,
    pub italic: bool,
    pub glyphs: u32,
    pub cell: (u16, u16, u16),
    pub vertical: crate::catalog::VerticalMetrics,
}
/// Experimental cumulative native-work budgets, never production C7 defaults.
/// Context/range UTF-8 bytes and calls are charged before each native attempt,
/// including fallback and context reshapes. Actual raw glyphs are charged after
/// HB returns and before any glyph publication (their count is not known sooner).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NativeWorkExperiment {
    pub calls: u64,
    pub context_bytes: u64,
    pub range_bytes: u64,
    pub raw_glyphs: u64,
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
    work: Option<(NativeWorkExperiment, NativeWorkExperiment)>,
    _local: PhantomData<Rc<()>>,
}
impl FontContext {
    /// Isolated experimental accounting. No renderer path enables it.
    pub fn begin_native_work_experiment(&mut self, limits: NativeWorkExperiment) {
        self.work = Some((limits, NativeWorkExperiment::default()));
    }
    pub fn native_work_experiment(&self) -> Option<NativeWorkExperiment> {
        self.work.map(|(_, used)| used)
    }
    fn preflight_native_work(&mut self, context: u64, range: u64) -> Result<(), FontError> {
        if let Some((limits, used)) = &mut self.work {
            let fail = || {
                FontError::new(
                    ErrorCode::LimitExceeded,
                    "experimental native-work budget exceeded",
                )
            };
            let next = NativeWorkExperiment {
                calls: used.calls.checked_add(1).ok_or_else(fail)?,
                context_bytes: used.context_bytes.checked_add(context).ok_or_else(fail)?,
                range_bytes: used.range_bytes.checked_add(range).ok_or_else(fail)?,
                raw_glyphs: used.raw_glyphs,
            };
            if next.calls > limits.calls
                || next.context_bytes > limits.context_bytes
                || next.range_bytes > limits.range_bytes
                || next.raw_glyphs > limits.raw_glyphs
            {
                return Err(fail());
            }
            *used = next;
        }
        Ok(())
    }
    fn record_native_glyphs(&mut self, count: u64) -> Result<(), FontError> {
        if let Some((limits, used)) = &mut self.work {
            let fail = || {
                FontError::new(
                    ErrorCode::LimitExceeded,
                    "experimental raw-glyph budget exceeded",
                )
            };
            used.raw_glyphs = used.raw_glyphs.checked_add(count).ok_or_else(fail)?;
            if used.raw_glyphs > limits.raw_glyphs {
                return Err(fail());
            }
        }
        Ok(())
    }
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
            work: None,
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
    fn probe_face_count(&self, bytes: Arc<[u8]>) -> Result<u32, FontError> {
        let probe = self.open(bytes, -1)?;
        // SAFETY: live initialized probe returned by FreeType.
        let count = unsafe { (*probe.raw).num_faces };
        u32::try_from(count)
            .ok()
            .filter(|&count| count > 0)
            .ok_or_else(|| {
                FontError::new(
                    ErrorCode::NativeFailure,
                    "invalid native collection face count",
                )
            })
    }
    /// Probe an in-memory font resource without publishing a native handle.
    /// Every reported face still requires ordinary catalog validation.
    pub fn face_count(&mut self, bytes: Arc<[u8]>) -> Result<u32, FontError> {
        self.check_bytes(&bytes)?;
        let count = self.probe_face_count(bytes)?;
        // Higher FreeType index bits select named instances, not TTC faces.
        if count > 0x1_0000 {
            return Err(FontError::new(
                ErrorCode::LimitExceeded,
                "collection face count exceeds supported native index range",
            ));
        }
        Ok(count)
    }
    fn with_face<T>(
        &mut self,
        bytes: Arc<[u8]>,
        index: u32,
        read: impl FnOnce(&Face<'_>) -> Result<T, FontError>,
    ) -> Result<T, FontError> {
        let count = self.probe_face_count(bytes.clone())?;
        if index > 0xffff || index >= count {
            return Err(FontError::new(
                ErrorCode::InvalidArgument,
                "collection face index out of range or named-instance selector",
            ));
        }
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
        if byte_limit as u64 > self.limits.font_bytes() || cost.is_none() {
            return Err(FontError::new(
                ErrorCode::LimitExceeded,
                "native intake reservation exceeded",
            ));
        }
        if cost.is_some_and(|n| n > self.limits.transient_bytes()) {
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
                let mut aliases = Vec::new();
                let mut name_bytes = 0_usize;
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
                    // Microsoft name strings are UTF-16BE even in older fonts
                    // whose platform encoding names a legacy glyph charset.
                    // Full and PostScript names are aliases, never substitutes
                    // for the preferred family or independent fake faces.
                    if !matches!(name.name, 1 | 4 | 6 | 16 | 21) || !matches!(name.platform, 0 | 3)
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
                    if !aliases.contains(&family) {
                        // Name records may reference overlapping storage. Bound
                        // aggregate decoded metadata, not just each record.
                        name_bytes = name_bytes.checked_add(family.len()).ok_or_else(|| {
                            FontError::new(ErrorCode::LimitExceeded, "font alias size overflow")
                        })?;
                        if name_bytes > byte_limit.saturating_mul(3) {
                            return Err(FontError::new(
                                ErrorCode::LimitExceeded,
                                "font alias budget exceeded",
                            ));
                        }
                        aliases.try_reserve(1).map_err(|_| {
                            FontError::new(
                                ErrorCode::AllocationFailed,
                                "font alias allocation failed",
                            )
                        })?;
                        aliases.push(family.clone());
                    }
                    let rank = (
                        u8::from(name.name != 16),
                        u8::from(name.language != 0x409),
                        name.language,
                    );
                    if matches!(name.name, 1 | 16)
                        && best.as_ref().is_none_or(|(old, _)| rank < *old)
                    {
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
                if aliases.is_empty() {
                    aliases.push(family.clone());
                }
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
                // FreeType validates the SFNT directory. Read only the fixed
                // OS/2 Windows metrics bytes; no native TT_OS2 layout assumption.
                let mut table = [0_u8; 78];
                let mut length = table.len() as ft::FT_ULong;
                let valid_win = FT_Load_Sfnt_Table(
                    face.raw,
                    u32::from_be_bytes(*b"OS/2") as ft::FT_ULong,
                    0,
                    table.as_mut_ptr(),
                    &mut length,
                ) == 0
                    && length == 78;
                let (ascent, descent) = if valid_win {
                    (
                        u16::from_be_bytes([table[74], table[75]]),
                        u16::from_be_bytes([table[76], table[77]]),
                    )
                } else {
                    // Non-SFNT faces retain their validated native cell metrics.
                    // This fallback is not a claim about GDI's font mapper.
                    (
                        u16::try_from((*face.raw).ascender)
                            .map_err(|_| native_error(0, "invalid cell ascent"))?,
                        u16::try_from(-i32::from((*face.raw).descender))
                            .map_err(|_| native_error(0, "invalid cell descent"))?,
                    )
                };
                let upem = (*face.raw).units_per_EM;
                let typo_descent = if valid_win {
                    i16::from_be_bytes([table[70], table[71]])
                } else {
                    (*face.raw).descender
                };
                let format = FT_Get_Font_Format(face.raw);
                let cff = !format.is_null() && CStr::from_ptr(format).to_bytes() == b"CFF";
                let (va, vd) = if cff {
                    (
                        u16::try_from(
                            (*face.raw)
                                .bbox
                                .xMax
                                .checked_add(typo_descent as ft::FT_Pos)
                                .ok_or_else(|| native_error(0, "vertical cell overflow"))?
                                .max(0),
                        )
                        .map_err(|_| native_error(0, "vertical cell ascent"))?,
                        u16::try_from(
                            (*face.raw)
                                .bbox
                                .xMin
                                .checked_add(typo_descent as ft::FT_Pos)
                                .and_then(|n| n.checked_neg())
                                .ok_or_else(|| native_error(0, "vertical cell overflow"))?
                                .max(0),
                        )
                        .map_err(|_| native_error(0, "vertical cell descent"))?,
                    )
                } else {
                    (ascent, descent)
                };
                if upem == 0 || u32::from(ascent) + u32::from(descent) == 0 {
                    return Err(native_error(0, "invalid ASS font cell metrics"));
                }
                Ok(Metadata {
                    family,
                    aliases,
                    weight,
                    italic: (*face.raw).style_flags & ft::FT_STYLE_FLAG_ITALIC != 0,
                    glyphs,
                    cell: (upem, ascent, descent),
                    vertical: crate::catalog::VerticalMetrics {
                        cff,
                        typo_descent,
                        ascent: va,
                        descent: vd,
                    },
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
    fn raster_native_compound_outline_survives_font_and_raster_owners() {
        use sha2::{Digest, Sha256};
        let limits = Limits::default()
            .with_transient_bytes(512 * 1024 * 1024)
            .unwrap();
        let path = std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
            .join("taiyou-shaping-fonts/indic.ttf");
        let bytes: Arc<[u8]> = std::fs::read(path).unwrap().into();
        let blob = FontBlob::from_unverified(FontId::new(Sha256::digest(&bytes).into(), 0), bytes);
        let mut font = FontContext::with_limits(limits.clone()).unwrap();
        let glyph = font
            .with_face(blob.shared_bytes(), 0, |face| {
                // SAFETY: scoped initialized face. FreeType identifies the compound;
                // no custom TrueType parser or shared native face is involved.
                unsafe {
                    let count = u32::try_from((*face.raw).num_glyphs).unwrap();
                    for glyph in 0..count.min(16_384) {
                        let code = ft::FT_Load_Glyph(
                            face.raw,
                            glyph,
                            ft::FT_LOAD_NO_RECURSE | ft::FT_LOAD_NO_BITMAP,
                        );
                        if code == 0 && (*(*face.raw).glyph).format == ft::FT_GLYPH_FORMAT_COMPOSITE
                        {
                            return Ok(glyph);
                        }
                    }
                }
                Err(FontError::new(
                    ErrorCode::FontUnavailable,
                    "pinned compound fixture missing",
                ))
            })
            .unwrap();
        let outline = font.outline(&blob, glyph, &limits).unwrap();
        drop(font);
        drop(blob);
        let mut minimum = (i32::MAX, i32::MAX);
        let mut maximum = (i32::MIN, i32::MIN);
        let mut ops = Vec::new();
        for &op in outline.outline().ops() {
            ops.push(
                op.try_map(|p| {
                    let scale = 20.0 / f64::from(outline.units_per_em());
                    let x = Q6::from_pixels(FiniteF64::new(p.x().get() * scale).unwrap()).unwrap();
                    let y = Q6::from_pixels(FiniteF64::new(-p.y().get() * scale).unwrap()).unwrap();
                    minimum = (minimum.0.min(x.ticks()), minimum.1.min(y.ticks()));
                    maximum = (maximum.0.max(x.ticks()), maximum.1.max(y.ticks()));
                    Ok(DevicePoint::new(x, y))
                })
                .unwrap(),
            );
        }
        let ceil = |v: i32| i32::try_from(-(-i64::from(v)).div_euclid(64)).unwrap();
        let bounds = PixelRect::new(
            minimum.0.div_euclid(64),
            minimum.1.div_euclid(64),
            ceil(maximum.0),
            ceil(maximum.1),
        )
        .unwrap();
        let mask = {
            let mut raster = RasterContext::with_limits(limits).unwrap();
            raster
                .rasterize_path(&ops, bounds, bounds.width() as usize)
                .unwrap()
        };
        let again = RasterContext::new()
            .unwrap()
            .rasterize_path(&ops, bounds, bounds.width() as usize)
            .unwrap();
        assert_eq!(mask.data(), again.data());
        drop(ops);
        drop(outline);
        assert!(mask.data().iter().any(|&v| v != 0));
    }
    #[test]
    fn raster_native_span_validation_and_lifetime() {
        let mut state = SpanCopy {
            data: vec![0; 8],
            width: 2,
            height: 2,
            stride: 4,
            failed: false,
            delivered: 0,
        };
        let good = RasterSpan {
            x: 0,
            len: 2,
            coverage: 255,
        };
        copy_spans(0, 1, &good, (&mut state as *mut SpanCopy).cast());
        assert_eq!(state.data, [0, 0, 0, 0, 255, 255, 0, 0]);
        let bad = RasterSpan {
            x: 2,
            len: 1,
            coverage: 255,
        };
        copy_spans(1, 1, &bad, (&mut state as *mut SpanCopy).cast());
        assert!(state.failed);
        assert_eq!(state.data, [0, 0, 0, 0, 255, 255, 0, 0]);
        for (y, count) in [(-1, 1), (2, 1), (0, -1), (0, 17)] {
            let mut s = SpanCopy {
                data: vec![0; 4],
                width: 2,
                height: 2,
                stride: 2,
                failed: false,
                delivered: 0,
            };
            copy_spans(y, count, &good, (&mut s as *mut SpanCopy).cast());
            assert!(s.failed);
            assert_eq!(s.data, [0; 4]);
        }
        let mut tail = SpanCopy {
            data: vec![0],
            width: 1,
            height: 1,
            stride: 1,
            failed: false,
            delivered: 0,
        };
        let spans = [
            RasterSpan {
                x: 0,
                len: 1,
                coverage: 255,
            },
            RasterSpan {
                x: 1,
                len: 0,
                coverage: 255,
            },
        ];
        copy_spans(0, 2, spans.as_ptr(), (&mut tail as *mut SpanCopy).cast());
        assert!(!tail.failed);
        assert_eq!(tail.data, [255]);
        assert_eq!(std::mem::size_of::<RasterSpan>(), 6);
        assert_eq!(std::mem::offset_of!(RasterSpan, coverage), 4);
        assert_eq!(
            std::mem::size_of::<RasterParams>(),
            8 * std::mem::size_of::<usize>() + 4 * std::mem::size_of::<ft::FT_Pos>()
        );
        assert_eq!(
            std::mem::offset_of!(RasterParams, clip_box),
            8 * std::mem::size_of::<usize>()
        );
        let path = [
            PathOp::MoveTo(DevicePoint::new(Q6::from_ticks(0), Q6::from_ticks(0))),
            PathOp::LineTo(DevicePoint::new(Q6::from_ticks(64), Q6::from_ticks(0))),
            PathOp::LineTo(DevicePoint::new(Q6::from_ticks(64), Q6::from_ticks(64))),
            PathOp::LineTo(DevicePoint::new(Q6::from_ticks(0), Q6::from_ticks(64))),
            PathOp::Close,
        ];
        let mask = {
            let mut ctx = RasterContext::new().unwrap();
            ctx.rasterize_path(&path, PixelRect::new(0, 0, 1, 1).unwrap(), 4)
                .unwrap()
        };
        assert_eq!(mask.data(), [255, 0, 0, 0]);
        assert_eq!(mask.stride(), 4);
    }
    #[test]
    fn geometry_native_outlines_are_owned_curves_and_checked() {
        use sha2::{Digest, Sha256};
        let limits = Limits::default()
            .with_transient_bytes(512 * 1024 * 1024)
            .unwrap();
        let read = |path: String, index| {
            let bytes: Arc<[u8]> = std::fs::read(path).unwrap().into();
            FontBlob::from_unverified(FontId::new(Sha256::digest(&bytes).into(), index), bytes)
        };
        let blob = read(std::env::var("TAIYOU_TEST_FONT").unwrap(), 0);
        fn request(text: &str, script: [u8; 4], rtl: bool) -> NativeRequest<'_> {
            NativeRequest {
                text,
                context: 0..text.len(),
                range: 0..text.len(),
                rtl,
                script: u32::from_be_bytes(script),
                language: "und",
                ligatures: true,
                vertical: false,
            }
        }
        let retained = {
            let mut ctx = FontContext::with_limits(limits.clone()).unwrap();
            let raw = ctx
                .shape_raw(&blob, &request("O", *b"Latn", false), &limits)
                .unwrap();
            let path = ctx.outline(&blob, raw.glyphs[0].glyph, &limits).unwrap();
            assert!(
                path.outline()
                    .ops()
                    .iter()
                    .any(|op| matches!(op, PathOp::CubicTo(..)))
            );
            assert!(
                path.outline()
                    .ops()
                    .iter()
                    .filter(|op| matches!(op, PathOp::Close))
                    .count()
                    >= 2
            );
            let space = ctx
                .shape_raw(&blob, &request(" ", *b"Latn", false), &limits)
                .unwrap();
            assert!(
                ctx.outline(&blob, space.glyphs[0].glyph, &limits)
                    .unwrap()
                    .outline()
                    .ops()
                    .is_empty()
            );
            assert!(ctx.outline(&blob, u32::MAX, &limits).is_err());
            assert!(
                ctx.outline(
                    &blob,
                    raw.glyphs[0].glyph,
                    &limits.clone().with_path_segments_per_frame(1).unwrap()
                )
                .is_err()
            );
            assert_eq!(LIVE.with(|n| n.get()), (1, 0));
            path
        };
        assert_eq!(LIVE.with(|n| n.get()), (0, 0));
        let mut fresh = FontContext::with_limits(limits.clone()).unwrap();
        assert_eq!(
            retained,
            fresh.outline(&blob, retained.glyph(), &limits).unwrap()
        );
        let ttf = read(
            std::path::PathBuf::from(std::env::var("RUNNER_TEMP").unwrap())
                .join("taiyou-shaping-fonts/indic.ttf")
                .to_str()
                .unwrap()
                .to_owned(),
            0,
        );
        let raw = fresh
            .shape_raw(&ttf, &request("क", *b"Deva", false), &limits)
            .unwrap();
        let path = fresh.outline(&ttf, raw.glyphs[0].glyph, &limits).unwrap();
        assert!(
            path.outline()
                .ops()
                .iter()
                .any(|op| matches!(op, PathOp::QuadTo(..)))
        );
        let compound = fresh
            .with_face(ttf.shared_bytes(), 0, |face| {
                // SAFETY: test-only inspection of this scoped face. NO_RECURSE asks
                // FreeType to identify a real composite; no custom glyf parser.
                unsafe {
                    let count = u32::try_from((*face.raw).num_glyphs).unwrap();
                    for glyph in 0..count.min(16_384) {
                        let code = ft::FT_Load_Glyph(
                            face.raw,
                            glyph,
                            ft::FT_LOAD_NO_RECURSE | ft::FT_LOAD_NO_BITMAP,
                        );
                        if code == 0 && (*(*face.raw).glyph).format == ft::FT_GLYPH_FORMAT_COMPOSITE
                        {
                            return Ok(glyph);
                        }
                    }
                }
                Err(FontError::new(
                    ErrorCode::FontUnavailable,
                    "pinned Indic fixture must contain a compound glyph",
                ))
            })
            .unwrap();
        let compound = fresh.outline(&ttf, compound, &limits).unwrap();
        assert!(!compound.outline().ops().is_empty());
        assert_eq!(compound.outline().ops().last(), Some(&PathOp::Close));
        let forged = FontBlob::from_unverified(FontId::new([0; 32], 0), blob.shared_bytes());
        assert!(fresh.outline(&forged, retained.glyph(), &limits).is_err());
        let wrong_face = FontBlob::from_unverified(
            FontId::new(*blob.id().sha256(), u32::MAX),
            blob.shared_bytes(),
        );
        assert!(
            fresh
                .outline(&wrong_face, retained.glyph(), &limits)
                .is_err()
        );
        let collection = std::env::var("TAIYOU_TEST_COLLECTION").unwrap();
        let a = read(collection.clone(), 0);
        let b = read(collection, 1);
        let a_raw = fresh
            .shape_raw(&a, &request("O", *b"Latn", false), &limits)
            .unwrap();
        let b_raw = fresh
            .shape_raw(&b, &request("O", *b"Latn", false), &limits)
            .unwrap();
        let a = fresh.outline(&a, a_raw.glyphs[0].glyph, &limits).unwrap();
        let b = fresh.outline(&b, b_raw.glyphs[0].glyph, &limits).unwrap();
        assert_ne!(a.font().face_index(), b.font().face_index());
        assert_ne!(a.outline(), b.outline());
        assert_eq!(
            std::mem::size_of::<OutlineFuncs>(),
            4 * std::mem::size_of::<usize>() + std::mem::size_of::<ft::FT_Pos>() * 2
        );
        assert_eq!(
            std::mem::offset_of!(OutlineFuncs, shift),
            4 * std::mem::size_of::<usize>()
        );
        assert_eq!(
            std::mem::offset_of!(OutlineFuncs, delta),
            std::mem::size_of::<OutlineFuncs>() - std::mem::size_of::<ft::FT_Pos>()
        );
        assert_eq!(
            std::mem::size_of::<ft::FT_Vector>(),
            2 * std::mem::size_of::<ft::FT_Pos>()
        );
    }
    #[test]
    fn geometry_native_copy_failure_stays_inside_callback_boundary() {
        assert!(check_outline_face(0).is_ok());
        assert!(check_outline_face(ft::FT_FACE_FLAG_TRICKY).is_err());
        let mut state = OutlineCopy {
            ops: Vec::new(),
            failed: false,
            open: false,
        };
        let point = ft::FT_Vector { x: 0, y: 0 };
        let user = (&mut state as *mut OutlineCopy).cast();
        assert_eq!(copy_move(&point, user), 1);
        assert!(state.failed);
        assert!(state.ops.is_empty());
        assert_eq!(copy_line(ptr::null(), user), 1);
        assert_eq!(copy_move(&point, ptr::null_mut()), 1);
    }
    #[test]
    fn geometry_native_implied_quadratic_endpoints_retain_half_units() {
        let mut points = [
            ft::FT_Vector { x: 0, y: 0 },
            ft::FT_Vector { x: 1, y: 1 },
            ft::FT_Vector { x: 2, y: 0 },
        ];
        let mut tags = [1, 0, 0];
        let mut contours = [2];
        let outline = ft::FT_Outline {
            n_contours: 1,
            n_points: 3,
            points: points.as_mut_ptr(),
            tags: tags.as_mut_ptr(),
            contours: contours.as_mut_ptr(),
            flags: 0,
        };
        let mut state = OutlineCopy {
            ops: Vec::with_capacity(8),
            failed: false,
            open: false,
        };
        for p in &points {
            check_outline_point(p).unwrap();
        }
        // SAFETY: all native arrays above have validated extents and remain live;
        // the callback state is exclusive for this synchronous decomposition.
        assert_eq!(
            unsafe {
                FT_Outline_Decompose(
                    &outline,
                    &outline_funcs(),
                    (&mut state as *mut OutlineCopy).cast(),
                )
            },
            0
        );
        state.close();
        assert!(!state.failed);
        assert_eq!(
            state.ops[1],
            PathOp::QuadTo(
                FontPoint::new(1.0, 1.0).unwrap(),
                FontPoint::new(1.5, 0.5).unwrap()
            )
        );
        assert!(
            check_outline_point(&ft::FT_Vector {
                x: 2_000_000_000,
                y: 0
            })
            .is_err()
        );
    }
    #[test]
    fn harfbuzz_abi_owned_results_and_rejected_requests_cleanup() {
        use sha2::{Digest, Sha256};
        assert_eq!(std::mem::size_of::<hb::hb_glyph_info_t>(), 20);
        assert_eq!(std::mem::size_of::<hb::hb_glyph_position_t>(), 20);
        assert_eq!(std::mem::size_of::<hb::hb_font_extents_t>(), 48);
        assert_eq!(std::mem::size_of::<hb::hb_glyph_extents_t>(), 16);
        assert_eq!(std::mem::size_of::<hb::hb_feature_t>(), 16);
        let bytes: Arc<[u8]> = std::fs::read(std::env::var("TAIYOU_TEST_FONT").unwrap())
            .unwrap()
            .into();
        let hash: [u8; 32] = Sha256::digest(&bytes).into();
        let invalid_face = FontBlob::from_unverified(FontId::new(hash, u32::MAX), bytes.clone());
        let valid = FontBlob::from_unverified(FontId::new(hash, 0), bytes);
        let junk: Arc<[u8]> = Arc::from([0_u8; 32]);
        let malformed =
            FontBlob::from_unverified(FontId::new(Sha256::digest(&junk).into(), 0), junk);
        let request = NativeRequest {
            text: "fi",
            context: 0..2,
            range: 0..2,
            rtl: false,
            script: u32::from_be_bytes(*b"Latn"),
            language: "en",
            ligatures: true,
            vertical: false,
        };
        let retained = {
            let mut context = FontContext::new().unwrap();
            for _ in 0..16 {
                assert!(
                    context
                        .shape_raw(&invalid_face, &request, &Limits::default())
                        .is_err()
                );
                assert!(
                    context
                        .shape_raw(&malformed, &request, &Limits::default())
                        .is_err()
                );
            }
            context
                .shape_raw(&valid, &request, &Limits::default())
                .unwrap()
        };
        assert!(retained.glyphs.iter().all(|g| g.glyph != 0));
        assert_eq!(retained.upem, 1000);
    }
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
