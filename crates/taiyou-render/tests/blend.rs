use std::sync::Arc;
use taiyou_render::{blend_pixel, compose, mul255};
use taiyou_types::*;

fn mask(bounds: PixelRect, stride: usize, bytes: &[u8]) -> Arc<Mask> {
    Arc::new(Mask::new(bounds, stride, Arc::from(bytes)).unwrap())
}
fn full() -> Arc<Mask> {
    mask(PixelRect::new(0, 0, 1, 1).unwrap(), 1, &[255])
}
fn op(
    index: u32,
    color: Rgba8,
    mask: Arc<Mask>,
    offset: PixelOffset,
    clip: Arc<ResolvedClip>,
) -> DrawOp {
    DrawOp::new(
        DrawOrder::new(0, EventId::new(0), 2, index),
        mask,
        offset,
        Paint::Solid(color),
        PaintSpace::Device,
        PaintMap::Identity,
        clip,
    )
}
fn render(size: Size, operations: &[DrawOp]) -> Frame {
    let limits = Limits::default();
    compose(size, operations, &limits, &mut OutputBudget::new(&limits)).unwrap()
}

#[test]
fn blend_exact_exhaustive_mul255_and_contract_colors() {
    for x in 0..=255_u8 {
        for y in 0..=255_u8 {
            let expected = (u64::from(x) * u64::from(y) + 127) / 255;
            assert_eq!(u64::from(mul255(x, y)), expected);
        }
    }
    let mut opaque_blue = [0, 0, 255, 255];
    blend_pixel(&mut opaque_blue, Rgba8::new(255, 0, 0, 128), 255, 255);
    assert_eq!(opaque_blue, [128, 0, 127, 255]);
    let mut clear = [0; 4];
    blend_pixel(&mut clear, Rgba8::new(255, 0, 0, 128), 255, 255);
    assert_eq!(clear, [128, 0, 0, 128]);
    let mut clipped = [0; 4];
    blend_pixel(&mut clipped, Rgba8::new(255, 0, 0, 128), 128, 128);
    assert_eq!(clipped, [32, 0, 0, 32]);
}

#[test]
fn blend_preserves_premultiplication_transparency_and_byte_bounds() {
    for alpha in 0..=255_u8 {
        for mask in [0, 1, 127, 128, 254, 255] {
            for clip in [0, 1, 127, 128, 254, 255] {
                let original = [alpha, alpha / 2, 0, alpha];
                let mut dst = original;
                blend_pixel(&mut dst, Rgba8::new(17, 128, 255, alpha), mask, clip);
                assert!(dst[..3].iter().all(|c| *c <= dst[3]));
                if alpha == 0 || mask == 0 || clip == 0 {
                    assert_eq!(dst, original);
                }
                let mut dst = original;
                blend_pixel(&mut dst, Rgba8::new(255, 255, 255, 0), mask, clip);
                assert_eq!(dst, original);
            }
        }
    }
}

#[test]
fn blend_order_is_observable_and_duplicate_or_unsorted_keys_fail() {
    let no_clip = Arc::new(ResolvedClip::None);
    let red = Rgba8::new(255, 0, 0, 128);
    let blue = Rgba8::new(0, 0, 255, 128);
    let a = [
        op(0, red, full(), PixelOffset::default(), no_clip.clone()),
        op(1, blue, full(), PixelOffset::default(), no_clip.clone()),
    ];
    let b = [
        op(0, blue, full(), PixelOffset::default(), no_clip.clone()),
        op(1, red, full(), PixelOffset::default(), no_clip),
    ];
    assert_eq!(render(Size::new(1, 1), &a).pixels(), [64, 0, 128, 192]);
    assert_eq!(render(Size::new(1, 1), &b).pixels(), [128, 0, 64, 192]);
    let limits = Limits::default();
    let mut budget = OutputBudget::new(&limits);
    assert_eq!(
        compose(
            Size::new(1, 1),
            &[a[0].clone(), a[0].clone()],
            &limits,
            &mut budget
        )
        .unwrap_err()
        .code(),
        ErrorCode::InternalError
    );
    assert_eq!(
        compose(
            Size::new(1, 1),
            &[a[1].clone(), a[0].clone()],
            &limits,
            &mut budget
        )
        .unwrap_err()
        .code(),
        ErrorCode::InvalidArgument
    );
    assert_eq!(budget.retained_bytes(), 0);
}

#[test]
fn compose_bounds_negative_origins_offsets_and_padding_are_safe() {
    let coverage = mask(
        PixelRect::new(-1, -1, 1, 1).unwrap(),
        3,
        &[0, 0, 0, 0, 128, 0],
    );
    let draw = op(
        0,
        Rgba8::new(255, 0, 0, 255),
        coverage,
        PixelOffset::default(),
        Arc::new(ResolvedClip::None),
    );
    let frame = render(Size::new(2, 2), &[draw]);
    assert_eq!(frame.stride(), 8);
    assert_eq!(
        frame.pixels(),
        [128, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    let offscreen = op(
        0,
        Rgba8::new(255, 0, 0, 255),
        full(),
        PixelOffset::new(i32::MAX, i32::MIN),
        Arc::new(ResolvedClip::None),
    );
    assert_eq!(render(Size::new(1, 1), &[offscreen]).pixels(), [0; 4]);
    assert_eq!(render(Size::new(2, 1), &[]).pixels(), [0; 8]);
    assert!(
        Mask::new(
            PixelRect::new(0, 0, 1, 2).unwrap(),
            usize::MAX,
            Arc::from([])
        )
        .is_err()
    );
    assert!(Mask::new(PixelRect::new(0, 0, 1, 1).unwrap(), 2, Arc::from([255, 1])).is_err());
    assert!(Mask::new(PixelRect::new(0, 0, 2, 1).unwrap(), 1, Arc::from([255])).is_err());
    assert!(Mask::new(PixelRect::new(0, 0, 0, 1).unwrap(), 0, Arc::from([])).is_err());
    let empty = mask(PixelRect::new(0, 0, 0, 0).unwrap(), 0, &[]);
    assert_eq!(
        empty.coverage_at(i64::MIN, i64::MAX, PixelOffset::new(i32::MAX, i32::MIN)),
        0
    );
}

#[test]
fn compose_clip_coverage_is_multiplied_once_and_inverse_includes_outside_mask() {
    let coverage = mask(PixelRect::new(0, 0, 2, 1).unwrap(), 2, &[255, 255]);
    let clip = mask(PixelRect::new(0, 0, 1, 1).unwrap(), 1, &[128]);
    for (inverse, expected) in [
        (false, [128, 0, 0, 128, 0, 0, 0, 0]),
        (true, [127, 0, 0, 127, 255, 0, 0, 255]),
    ] {
        let clip = Arc::new(ResolvedClip::MaskCoverage {
            mask: clip.clone(),
            offset: PixelOffset::default(),
            inverse,
        });
        let draw = op(
            0,
            Rgba8::new(255, 0, 0, 255),
            coverage.clone(),
            PixelOffset::default(),
            clip,
        );
        assert_eq!(render(Size::new(2, 1), &[draw]).pixels(), expected);
    }
}

#[test]
fn compose_output_limit_tracks_retained_clones_and_releases_on_final_drop() {
    let limits = Limits::default().with_output_bytes(4).unwrap();
    let mut budget = OutputBudget::new(&limits);
    let frame = compose(Size::new(1, 1), &[], &limits, &mut budget).unwrap();
    let clone = frame.clone();
    assert_eq!(budget.retained_bytes(), 4);
    assert_eq!(frame.charged_bytes(), 4);
    assert_eq!(
        compose(Size::new(1, 1), &[], &limits, &mut budget)
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    drop(frame);
    assert_eq!(budget.retained_bytes(), 4);
    std::thread::spawn(move || drop(clone)).join().unwrap();
    assert_eq!(budget.retained_bytes(), 0);
    let frame = compose(Size::new(1, 1), &[], &limits, &mut budget).unwrap();
    drop(budget);
    assert_eq!(frame.pixels(), [0; 4]);
}

#[test]
fn compose_output_limit_errors_release_reservations_and_never_publish_partial_frames() {
    let limits = Limits::default().with_output_bytes(4).unwrap();
    let mut budget = OutputBudget::new(&limits);
    let reservation = budget.reserve(4).unwrap();
    assert!(
        Frame::from_owned_pixels(Size::new(1, 1), vec![255, 0, 0, 0], &limits, reservation)
            .is_err()
    );
    assert_eq!(budget.retained_bytes(), 0);
    assert!(compose(Size::new(0, 1), &[], &limits, &mut budget).is_err());
    assert!(compose(Size::new(2, 1), &[], &limits, &mut budget).is_err());
    let tiny_transient = limits.clone().with_transient_bytes(3).unwrap();
    assert_eq!(
        compose(Size::new(1, 1), &[], &tiny_transient, &mut budget)
            .unwrap_err()
            .code(),
        ErrorCode::LimitExceeded
    );
    assert_eq!(budget.retained_bytes(), 0);
}
