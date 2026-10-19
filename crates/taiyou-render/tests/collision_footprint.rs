use taiyou_render::collision_footprint::effect_bounds;
use taiyou_types::*;

fn b(a: [f64;4]) -> ScriptBounds { ScriptBounds::new(a[0],a[1],a[2],a[3]).unwrap() }
fn q(value: f64) -> Q6 { Q6::from_pixels(FiniteF64::new(value).unwrap()).unwrap() }

#[test]
fn approved_f1_expansion_shadow_visibility_and_fractional_parameters() {
    let ink = b([0.0,0.0,10.0,10.0]);
    for (radius, expected) in [(2.0,[-2.0,-2.0,12.0,12.0]),(5.0,[-5.0,-5.0,15.0,15.0]),(0.25,[-0.25,-0.25,10.25,10.25])] {
        assert_eq!(effect_bounds(ink,Some(q(radius)),None,true,true).unwrap(),Some(b(expected)));
    }
    let shadow = Some((q(3.5),q(-2.25)));
    assert_eq!(effect_bounds(ink,Some(q(2.0)),shadow,true,true).unwrap(),Some(b([-2.0,-4.25,15.5,12.0])));
    assert_eq!(effect_bounds(ink,Some(q(2.0)),None,true,false).unwrap(),Some(ink));
    assert_eq!(effect_bounds(ink,Some(q(2.0)),shadow,false,false).unwrap(),Some(b([1.5,-4.25,15.5,9.75])));
    assert_eq!(effect_bounds(ink,None,shadow,false,false).unwrap(),Some(b([3.5,-2.25,13.5,7.75])));
    assert_eq!(effect_bounds(ink,Some(q(2.0)),None,false,false).unwrap(),None);
    assert_eq!(effect_bounds(ink,None,Some((q(0.0),q(0.0))),false,false).unwrap(),Some(ink));
    assert_eq!(effect_bounds(b([-10.0,-10.0,-10.0,20.0]),Some(q(100.0)),shadow,true,true).unwrap(),None);
}

#[test]
fn approved_f1_two_border_widths_have_different_displacements() {
    use taiyou_render::collision::*;
    let mut shifts = Vec::new();
    for radius in [2.0,5.0] {
        let bounds = effect_bounds(b([0.0,0.0,10.0,10.0]),Some(q(radius)),None,true,true).unwrap().unwrap();
        let input = |id| CollisionInput { id: EventId::new(id), layer: 0, start: TimeUs::from_micros(0), alignment: Alignment::new(7).unwrap(), anchor: ScriptPoint::new(FiniteF64::new(0.0).unwrap(),FiniteF64::new(0.0).unwrap()), bounds, eligible: true };
        let placed = resolve_collisions(&[input(0),input(1)],Profile::CoreV1,&Limits::default()).unwrap();
        shifts.push(placed[1].anchor.y().get());
    }
    assert_eq!(shifts,[14.0,20.0]);
}
