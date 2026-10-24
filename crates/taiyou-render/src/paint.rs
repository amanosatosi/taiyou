use taiyou_types::Rgba8;

/// Exact C6 multiplication. The result is 0..255 for all byte inputs.
pub fn mul255(x: u8, y: u8) -> u8 {
    let value = (u32::from(x) * u32::from(y) + 127) / 255;
    // The proven byte-range integer is extracted without an unchecked cast.
    value.to_le_bytes()[0]
}

/// Scalar encoded-domain source-over. Destination must already be premultiplied.
/// Preserves the separate rounding boundaries specified by C6.
pub fn blend_pixel(dst: &mut [u8; 4], paint: Rgba8, mask: u8, clip: u8) {
    let coverage = mul255(mask, clip);
    let alpha = mul255(paint.a(), coverage);
    blend_alpha(dst, paint, alpha);
}
/// Rasterizer.cpp FillAlphaMashBody_c: paint alpha is multiplied in native
/// coverage units after Gaussian/BE truncation. Keep ordinary A8 callers intact.
pub(crate) fn blend_xy_pixel(dst: &mut [u8; 4], paint: Rgba8, mask: u8, clip: u8) {
    let alpha = ((u32::from(mask) * u32::from(paint.a()) + 32) >> 6) as u8;
    let alpha = u32::from(mul255(alpha, clip));
    // Rasterizer.cpp pixmix_c/packed_pix_mix_sse2 compose the complete
    // numerator before truncating: source*(alpha+1) + destination*(256-alpha).
    // The bitmap retains inverse alpha, so its update has the same divisor.
    // This rule is portable integer arithmetic, independent of GDI rendering.
    let inverse = 256 - alpha;
    for (dst, color) in dst[..3].iter_mut().zip([paint.r(), paint.g(), paint.b()]) {
        *dst = ((u32::from(*dst) * inverse + u32::from(color) * (alpha + 1)) >> 8) as u8;
    }
    dst[3] = 255 - (((255 - u32::from(dst[3])) * inverse) >> 8) as u8;
}
fn blend_alpha(dst: &mut [u8; 4], paint: Rgba8, alpha: u8) {
    let source = [
        mul255(paint.r(), alpha),
        mul255(paint.g(), alpha),
        mul255(paint.b(), alpha),
        alpha,
    ];
    for (dst, src) in dst.iter_mut().zip(source) {
        *dst = src.saturating_add(mul255(*dst, 255 - alpha));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xy_packed_mix_retains_native_truncation_and_inverse_alpha() {
        let mut clear = [0; 4];
        blend_xy_pixel(&mut clear, Rgba8::new(62, 17, 17, 98), 64, 255);
        // Direct drawImg capture at (949,684), where rounded /255 gave 24,7,7.
        assert_eq!(clear, [23, 6, 6, 98]);
        let mut clear = [0; 4];
        blend_xy_pixel(&mut clear, Rgba8::new(62, 17, 17, 253), 64, 255);
        assert_eq!(clear, [61, 16, 16, 253]);
        let mut overlap = [128; 4];
        blend_xy_pixel(&mut overlap, Rgba8::new(255, 255, 255, 1), 64, 255);
        assert_eq!(overlap, [129; 4]);
    }

    #[test]
    fn xy_packed_mix_preserves_premultiplication_and_zero_coverage() {
        for destination_alpha in 0..=255_u8 {
            for source_alpha in 0..=255_u8 {
                let original = [destination_alpha, destination_alpha / 2, 0, destination_alpha];
                let mut dst = original;
                blend_xy_pixel(&mut dst, Rgba8::new(17, 128, 255, source_alpha), 64, 255);
                assert!(dst[..3].iter().all(|&c| c <= dst[3]));
                assert!(dst[3] >= destination_alpha);
                if source_alpha == 0 {
                    assert_eq!(dst, original);
                }
                for (mask, clip) in [(0, 255), (64, 0)] {
                    let mut dst = original;
                    blend_xy_pixel(&mut dst, Rgba8::new(17, 128, 255, source_alpha), mask, clip);
                    assert_eq!(dst, original);
                }
            }
        }
    }
}
