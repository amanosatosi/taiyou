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
    let source = [mul255(paint.r(), alpha), mul255(paint.g(), alpha), mul255(paint.b(), alpha), alpha];
    for (dst, src) in dst.iter_mut().zip(source) {
        *dst = src.saturating_add(mul255(*dst, 255 - alpha));
    }
}
