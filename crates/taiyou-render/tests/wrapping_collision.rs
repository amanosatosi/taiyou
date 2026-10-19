use taiyou_render::collision::*;
use taiyou_types::*;

fn input(id: u32, alignment: u8, rect: [f64; 4]) -> CollisionInput {
    CollisionInput {
        id: EventId::new(id),
        layer: 0,
        start: TimeUs::from_micros(0),
        alignment: Alignment::new(alignment).unwrap(),
        anchor: ScriptPoint::new(FiniteF64::new(0.0).unwrap(), FiniteF64::new(0.0).unwrap()),
        bounds: ScriptBounds::new(rect[0], rect[1], rect[2], rect[3]).unwrap(),
        eligible: true,
    }
}
fn place(events: &[CollisionInput]) -> Vec<PlacedEvent> {
    resolve_collisions(events, Profile::CoreV1, &Limits::default()).unwrap()
}

#[test]
fn bottom_up_and_middle_top_down_minimum_contact() {
    for alignment in 1..=9 {
        let a = input(0, alignment, [0.0, 0.0, 10.0, 10.0]);
        let b = input(1, alignment, [0.0, 0.0, 10.0, 10.0]);
        let p = place(&[b, a]);
        assert_eq!(p[0].anchor.y().get(), 0.0);
        assert_eq!(p[1].anchor.y().get(), if alignment <= 3 { -10.0 } else { 10.0 });
        assert_eq!(p[0].id, EventId::new(0));
    }
}

#[test]
fn layers_explicit_and_nonoverlapping_rectangles_are_not_obstacles() {
    let a = input(0, 2, [0.0, 0.0, 10.0, 10.0]);
    for mut b in [a, input(1, 2, [10.0, 0.0, 20.0, 10.0]), input(1, 2, [0.0, 10.0, 10.0, 20.0])] {
        b.id = EventId::new(1);
        if b.bounds == a.bounds { b.layer = 1; }
        assert_eq!(place(&[a, b])[1].anchor.y().get(), 0.0);
    }
    let mut explicit = a;
    explicit.eligible = false;
    let b = input(1, 2, [0.0, 0.0, 10.0, 10.0]);
    assert_eq!(place(&[explicit, b])[1].anchor.y().get(), 0.0);
    explicit.id = EventId::new(2);
    assert_eq!(place(&[a, explicit])[1].anchor.y().get(), 0.0);
}

#[test]
fn start_then_id_order_is_independent_of_input_and_paint_order() {
    let mut a = input(0, 2, [0.0, 0.0, 10.0, 10.0]);
    a.start = TimeUs::from_micros(1);
    let b = input(1, 2, [0.0, 0.0, 10.0, 10.0]);
    let expected = place(&[a, b]);
    assert_eq!(expected[0].anchor.y().get(), -10.0);
    assert_eq!(expected[1].anchor.y().get(), 0.0);
    assert_eq!(place(&[b, a]), expected);
}

#[test]
fn rechecks_earlier_obstacles_after_later_displacement() {
    // A and B touch. C initially hits B only; clearing B enters A.
    let a = input(0, 7, [0.0, 10.0, 10.0, 20.0]);
    let b = input(1, 7, [0.0, 0.0, 10.0, 10.0]);
    let c = input(2, 7, [0.0, -5.0, 10.0, 5.0]);
    let p = place(&[a, b, c]);
    assert_eq!(p[2].bounds.y0().get(), 20.0);
    let a = input(0, 2, [0.0, -20.0, 10.0, -10.0]);
    let b = input(1, 2, [0.0, -10.0, 10.0, 0.0]);
    let c = input(2, 2, [0.0, -5.0, 10.0, 5.0]);
    assert_eq!(place(&[a, b, c])[2].bounds.y1().get(), -20.0);
}

#[test]
fn empty_boxes_fractional_anchors_and_offscreen_placement() {
    let a = input(0, 2, [0.0, 0.0, 0.0, 100.0]);
    let b = input(1, 2, [0.0, 0.0, 10.0, 10.0]);
    assert_eq!(place(&[a, b])[1].anchor.y().get(), 0.0);
    let mut events = Vec::new();
    for id in 0..200 {
        let mut e = input(id, 2, [-1.25, -2.5, 1.25, 0.0]);
        e.anchor = ScriptPoint::new(FiniteF64::new(-10.5).unwrap(), FiniteF64::new(0.25).unwrap());
        events.push(e);
    }
    let p = place(&events);
    assert_eq!(p[199].anchor.y().get(), 0.25 - 199.0 * 2.5);
    assert_eq!(p[199].anchor.x().get(), -10.5);
}

#[test]
fn rejects_duplicate_ids_overflow_limits_and_unapproved_profile() {
    let a = input(0, 2, [0.0, 0.0, 10.0, 10.0]);
    assert_eq!(resolve_collisions(&[a, a], Profile::CoreV1, &Limits::default()).unwrap_err().code(), ErrorCode::InvalidArgument);
    let tiny = Limits::default().with_transient_bytes(1).unwrap();
    assert_eq!(resolve_collisions(&[a], Profile::CoreV1, &tiny).unwrap_err().code(), ErrorCode::LimitExceeded);
    assert_eq!(resolve_collisions(&[a], Profile::LegacyXyV1, &Limits::default()).unwrap_err().code(), ErrorCode::UnsupportedFeature);
    let mut huge = input(1, 2, [0.0, 0.0, 1.0, f64::MAX]);
    huge.anchor = ScriptPoint::new(FiniteF64::new(0.0).unwrap(), FiniteF64::new(f64::MAX).unwrap());
    assert_eq!(resolve_collisions(&[huge], Profile::CoreV1, &Limits::default()).unwrap_err().code(), ErrorCode::LimitExceeded);
}
