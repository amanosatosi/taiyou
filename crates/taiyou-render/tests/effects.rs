use std::sync::Arc;
use taiyou_render::effects::{BlurSpec, apply_effects};
use taiyou_types::*;

fn spec(blur: f64, be: f64) -> BlurSpec {
    BlurSpec::new(
        FiniteF64::new(blur).unwrap(),
        FiniteF64::new(be).unwrap(),
        [1.0, 1.0],
        &Limits::default(),
    )
    .unwrap()
}
fn impulse() -> Arc<Mask> {
    let mut data = vec![0; 25];
    data[12] = 255;
    Arc::new(Mask::new(PixelRect::new(-2, -2, 3, 3).unwrap(), 5, data.into()).unwrap())
}
#[test]
fn threshold_zero_identity_and_source_immutability() {
    let mask = impulse();
    for (blur, be) in [
        (0.0, 0.0),
        (0.333332, 0.0),
        (f64::from(0.333333_f32), 0.0),
        (0.0, -2.0),
    ] {
        let output = apply_effects(&mask, &spec(blur, be), &Limits::default()).unwrap();
        assert!(Arc::ptr_eq(&mask, &output));
    }
    let blurred = apply_effects(&mask, &spec(0.333334, 0.0), &Limits::default()).unwrap();
    assert_ne!(blurred.bounds(), mask.bounds());
    assert_eq!(mask.data()[12], 255);
    assert_eq!(mask.data().iter().filter(|&&b| b != 0).count(), 1);
}
#[test]
fn gaussian_one_impulse_uses_reference_short_float_kernel() {
    let output = apply_effects(&impulse(), &spec(1.0, 0.0), &Limits::default()).unwrap();
    assert_eq!(spec(1.0, 0.0).support(), [1, 1]);
    let expected = [[19, 32, 19], [32, 52, 32], [19, 32, 19]];
    for (y, row) in expected.into_iter().enumerate() {
        for (x, v) in row.into_iter().enumerate() {
            assert_eq!(
                output.coverage_at(x as i64 - 1, y as i64 - 1, PixelOffset::default()),
                v
            );
        }
    }
}
#[test]
fn integer_fractional_edge_and_combined_filters_are_distinct() {
    let source = impulse();
    let one = apply_effects(&source, &spec(0.0, 1.0), &Limits::default()).unwrap();
    // Integer BE truncates a 3x3 outer product /16 once, not each axis.
    assert_eq!(one.coverage_at(0, 0, PixelOffset::default()), 63);
    assert_eq!(one.coverage_at(1, 0, PixelOffset::default()), 31);
    assert_eq!(one.coverage_at(1, 1, PixelOffset::default()), 15);
    let half = apply_effects(&source, &spec(0.0, 0.5), &Limits::default()).unwrap();
    assert!(half.coverage_at(0, 0, PixelOffset::default()) > 63);
    let two = apply_effects(&source, &spec(0.0, 2.0), &Limits::default()).unwrap();
    assert!(two.coverage_at(0, 0, PixelOffset::default()) < 63);
    let gaussian = apply_effects(&source, &spec(1.0, 0.0), &Limits::default()).unwrap();
    let separate = apply_effects(&gaussian, &spec(0.0, 1.5), &Limits::default()).unwrap();
    let combined = apply_effects(&source, &spec(1.0, 1.5), &Limits::default()).unwrap();
    assert_eq!(separate.bounds(), combined.bounds());
    assert_eq!(separate.data(), combined.data());
    // BE activation allows even sub-threshold positive Gaussian to execute.
    assert!(spec(0.2, 1.0).active());
}
#[test]
fn support_pass_dimension_and_transient_limits_fail_before_filtering() {
    for (blur, be) in [(1000.0, 0.0), (0.0, 65.0), (f64::MAX, 1.0), (0.0, f64::MAX)] {
        assert_eq!(
            BlurSpec::new(
                FiniteF64::new(blur).unwrap(),
                FiniteF64::new(be).unwrap(),
                [1.0, 1.0],
                &Limits::default()
            )
            .err()
            .unwrap()
            .code(),
            ErrorCode::LimitExceeded
        );
    }
    let s = spec(2.0, 1.0);
    let source = impulse();
    for limits in [
        Limits::default().with_max_dimension(5).unwrap(),
        Limits::default().with_transient_bytes(1).unwrap(),
        Limits::default().with_output_pixels(1).unwrap(),
    ] {
        assert_eq!(
            apply_effects(&source, &s, &limits).unwrap_err().code(),
            ErrorCode::LimitExceeded
        );
    }
    assert_eq!(source.data()[12], 255);
}

#[test]
fn be_preserves_reference_small_padding_and_unfiltered_allocation_ring() {
    let source =
        Arc::new(Mask::new(PixelRect::new(0, 0, 1, 1).unwrap(), 1, Arc::from([255])).unwrap());
    let one = apply_effects(&source, &spec(0.0, 1.0), &Limits::default()).unwrap();
    assert_eq!(one.bounds(), PixelRect::new(-1, -1, 2, 2).unwrap());
    assert_eq!(one.data(), &[0, 0, 0, 0, 63, 0, 0, 0, 0]);
    assert_eq!(spec(0.0, 4.0).support(), [1, 1]);
    assert_eq!(spec(0.0, 16.0).support(), [2, 2]);
    assert_eq!(source.data(), &[255]);
}

#[test]
fn gaussian_preserves_native_overlay_units_until_paint_conversion() {
    // Rasterizer::GaussianBlur -> xy_byte_2_float passes overlay bytes as-is.
    // At sigma=1 the independently normalized three-tap kernel gives this
    // rounded 64-unit impulse. Converting to A8 before filtering differs.
    let source = Arc::new(
        Mask::new_xy(PixelRect::new(0, 0, 1, 1).unwrap(), 1, Arc::from([64])).unwrap(),
    );
    let output = apply_effects(&source, &spec(1.0, 0.0), &Limits::default()).unwrap();
    assert!(output.is_xy_coverage());
    assert_eq!(output.bounds(), PixelRect::new(-1, -1, 2, 2).unwrap());
    assert_eq!(output.data(), &[5, 8, 5, 8, 13, 8, 5, 8, 5]);
    assert_eq!(output.coverage_a8_at(-1, -1, PixelOffset::default()), 20);
    assert_eq!(source.data(), &[64]);
    let a8 = apply_effects(&impulse(), &spec(1.0, 0.0), &Limits::default()).unwrap();
    assert_eq!(a8.coverage_at(-1, -1, PixelOffset::default()), 19);
}
