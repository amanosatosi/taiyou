use std::sync::Arc;
use taiyou_render::geometry::*;
use taiyou_types::*;
fn f(v:f64)->FiniteF64 { FiniteF64::new(v).unwrap() }
fn p(x:f64,y:f64)->FontPoint { FontPoint::new(x,y).unwrap() }
fn s(x:f64,y:f64)->ScriptPoint { ScriptPoint::new(f(x),f(y)) }
fn device()->DeviceMapping { DeviceMapping::new(Size::new(100,100),PixelRect::new(0,0,100,100).unwrap()).unwrap() }
fn mapping(scale:(f64,f64),at:(f64,f64),rotation:f64)->GeometryMap { GeometryMap::new((f(scale.0),f(scale.1)),s(0.0,0.0),s(at.0,at.1),None,f(rotation),device()).unwrap() }
fn square()->Arc<Outline> { Arc::new(Outline::new(vec![PathOp::MoveTo(p(0.0,0.0)),PathOp::LineTo(p(1.0,0.0)),PathOp::LineTo(p(1.0,1.0)),PathOp::LineTo(p(0.0,1.0)),PathOp::Close],&Limits::default()).unwrap()) }
fn xy(p:DevicePoint)->(i32,i32) { (p.x().ticks(),p.y().ticks()) }
#[test]
fn geometry_square_identity_and_integer_translation() {
    let a=map_outline(square(),mapping((1.0,1.0),(0.0,0.0),0.0),&Limits::default()).unwrap();
    assert_eq!(a.ops(),&[PathOp::MoveTo(DevicePoint::new(Q6::from_ticks(0),Q6::from_ticks(0))),PathOp::LineTo(DevicePoint::new(Q6::from_ticks(64),Q6::from_ticks(0))),PathOp::LineTo(DevicePoint::new(Q6::from_ticks(64),Q6::from_ticks(-64))),PathOp::LineTo(DevicePoint::new(Q6::from_ticks(0),Q6::from_ticks(-64))),PathOp::Close]);
    let b=map_outline(square(),mapping((1.0,1.0),(-500.0,200.0),0.0),&Limits::default()).unwrap();
    assert_eq!(a.ops(),b.ops()); assert_eq!(a.phase(),b.phase()); assert_eq!(b.offset(),PixelOffset::new(-500,200));
    assert_eq!(xy(b.bounds().unwrap().min()),(-32000,12736));
}
#[test]
fn geometry_rotation_order_and_nonuniform_device_mapping() {
    let m=GeometryMap::new((f(2.0),f(3.0)),s(4.0,5.0),s(10.0,20.0),None,f(90.0),DeviceMapping::new(Size::new(100,100),PixelRect::new(7,11,207,311).unwrap()).unwrap()).unwrap();
    // (1,1) -> (2,-3)+(4,5) -> (2,-6)+(10,20) -> (31,53).
    assert_eq!(xy(m.point(p(1.0,1.0)).unwrap()),(31*64,53*64));
    assert_eq!(xy(mapping((1.0,1.0),(0.0,0.0),90.0).point(p(1.0,0.0)).unwrap()),(0,-64));
    assert_eq!(xy(mapping((1.0,1.0),(0.0,0.0),-90.0).point(p(1.0,0.0)).unwrap()),(0,64));
    let explicit=GeometryMap::new((f(1.0),f(1.0)),s(0.0,0.0),s(10.0,0.0),Some(s(0.0,0.0)),f(90.0),device()).unwrap();
    assert_eq!(xy(explicit.point(p(1.0,0.0)).unwrap()),(0,-704));
}
#[test]
fn geometry_negative_phase_and_rounding_are_euclidean() {
    let a=map_outline(square(),mapping((1.0,1.0),(-1.0/64.0,0.25),0.0),&Limits::default()).unwrap();
    assert_eq!(a.offset(),PixelOffset::new(-1,0)); assert_eq!(a.phase(),(63,16));
    assert_eq!(a.ops()[0],PathOp::MoveTo(DevicePoint::new(Q6::from_ticks(63),Q6::from_ticks(16))));
    for (v,expected) in [(-1.0/128.0,-1),(1.0/128.0,1)] { assert_eq!(xy(mapping((1.0,1.0),(v,0.0),0.0).point(p(0.0,0.0)).unwrap()).0,expected); }
}
#[test]
fn geometry_preserves_quadratic_cubic_contours_and_nonzero_winding() {
    let path=Arc::new(Outline::new(vec![PathOp::MoveTo(p(0.0,0.0)),PathOp::QuadTo(p(1.0,2.0),p(2.0,0.0)),PathOp::Close,PathOp::MoveTo(p(3.0,0.0)),PathOp::CubicTo(p(4.0,1.0),p(5.0,-1.0),p(6.0,0.0)),PathOp::Close],&Limits::default()).unwrap());
    let mapped=map_outline(path.clone(),mapping((1.0,1.0),(0.0,0.0),0.0),&Limits::default()).unwrap();
    assert_eq!(path.fill_rule(),FillRule::Nonzero);
    assert_eq!(mapped.ops()[1],PathOp::QuadTo(DevicePoint::new(Q6::from_ticks(64),Q6::from_ticks(-128)),DevicePoint::new(Q6::from_ticks(128),Q6::from_ticks(0))));
    assert!(matches!(mapped.ops()[4],PathOp::CubicTo(..)));
    assert_eq!(mapped.ops().iter().filter(|op| matches!(op,PathOp::Close)).count(),2);
    // CCW y-up square becomes CW y-down; order is retained, not sorted/reversed.
    assert_eq!(square().ops()[2],PathOp::LineTo(p(1.0,1.0)));
}
#[test]
fn geometry_empty_and_malformed_paths_and_budget_rejection() {
    let empty=Arc::new(Outline::new(vec![],&Limits::default()).unwrap());
    let mapped=map_outline(empty,mapping((1.0,1.0),(0.0,0.0),0.0),&Limits::default()).unwrap();
    assert!(mapped.ops().is_empty()); assert_eq!(mapped.bounds(),None);
    for ops in [vec![PathOp::Close],vec![PathOp::LineTo(p(0.0,0.0))],vec![PathOp::MoveTo(p(0.0,0.0))]] { assert!(Outline::new(ops,&Limits::default()).is_err()); }
    assert!(FontPoint::new(f64::NAN,0.0).is_err()); assert!(FontPoint::new(0.0,f64::INFINITY).is_err());
    assert!(map_outline(square(),mapping((1.0,1.0),(f64::MAX,0.0),0.0),&Limits::default()).is_err());
    assert!(map_outline(square(),mapping((f64::MAX,1.0),(0.0,0.0),0.0),&Limits::default()).is_err());
    for limits in [Limits::default().with_path_segments_per_frame(1).unwrap(),Limits::default().with_transient_bytes(1).unwrap()] { assert!(map_outline(square(),mapping((1.0,1.0),(0.0,0.0),0.0),&limits).is_err()); }
    assert!(DeviceMapping::new(Size::new(0,1),PixelRect::new(0,0,1,1).unwrap()).is_err());
}
