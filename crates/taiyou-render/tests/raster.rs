#![cfg(feature="native")]
use std::sync::Arc;
use taiyou_render::{geometry::*,raster::*};
use taiyou_types::*;
fn f(v:f64)->FiniteF64 { FiniteF64::new(v).unwrap() }
fn p(x:f64,y:f64)->FontPoint { FontPoint::new(x,-y).unwrap() }
fn rectangle(x:f64,y:f64,w:f64,h:f64)->Vec<PathOp<FontPoint>> {
    vec![PathOp::MoveTo(p(x,y)),PathOp::LineTo(p(x+w,y)),PathOp::LineTo(p(x+w,y+h)),PathOp::LineTo(p(x,y+h)),PathOp::Close]
}
fn instance(ops:Vec<PathOp<FontPoint>>,x:f64,y:f64)->GeometryInstance {
    let outline=Arc::new(Outline::new(ops,&Limits::default()).unwrap());
    let map=GeometryMap::new((f(1.0),f(1.0)),ScriptPoint::new(f(0.0),f(0.0)),ScriptPoint::new(f(x),f(y)),None,f(0.0),DeviceMapping::new(Size::new(1,1),PixelRect::new(0,0,1,1).unwrap()).unwrap()).unwrap();
    map_outline(outline,map,&Limits::default()).unwrap()
}
fn group(instances:Vec<GeometryInstance>)->CoverageGroup { CoverageGroup::new(instances,CoveragePass::Fill,Profile::CoreV1,&Limits::default()).unwrap() }
fn draw(instances:Vec<GeometryInstance>)->Arc<Mask> { rasterize(&mut RasterContext::new().unwrap(),&group(instances)).unwrap() }
#[test]
fn raster_rect() {
    let a=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),0.0,0.0)]);
    assert_eq!(a.bounds(),PixelRect::new(0,0,1,1).unwrap()); assert_eq!(a.data(),[255]);
    let b=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),-3.0,-2.0)]);
    assert_eq!(b.bounds(),PixelRect::new(-3,-2,-2,-1).unwrap()); assert_eq!(a.data(),b.data());
    for ops in [vec![],rectangle(0.0,0.0,0.0,1.0),rectangle(0.0,0.0,1.0,0.0)] {
        let m=draw(vec![instance(ops,0.0,0.0)]); assert!(m.bounds().is_empty());assert_eq!(m.stride(),0);assert!(m.data().is_empty());
    }
    assert!(draw(vec![]).data().is_empty());
}
#[test]
fn raster_winding() {
    let a=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),0.0,0.0),instance(rectangle(0.0,0.0,1.0,1.0),0.0,0.0)]);
    assert_eq!(a.data(),[255]);
    let a=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),0.0,0.0),instance(rectangle(0.0,0.0,1.0,1.0),0.5,0.0)]);
    assert_eq!(a.data(),[255,128]);
    let mut ops=rectangle(0.0,0.0,3.0,3.0);
    ops.extend([PathOp::MoveTo(p(1.0,1.0)),PathOp::LineTo(p(1.0,2.0)),PathOp::LineTo(p(2.0,2.0)),PathOp::LineTo(p(2.0,1.0)),PathOp::Close]);
    assert_eq!(draw(vec![instance(ops,0.0,0.0)]).data(),[255,255,255,255,0,255,255,255,255]);
}
#[test]
fn raster_phase() {
    // Axis-aligned coverage is exact pixel intersection area, quantized to 256
    // levels by the pinned grayscale backend (full 256 saturates to 255).
    for (x,y,expected) in [(0.5,0.0,vec![128,128]),(0.0,0.5,vec![128,128]),(0.5,0.5,vec![64,64,64,64]),(-1.0/64.0,0.0,vec![4,252]),(-0.5,-0.5,vec![64,64,64,64])] {
        let a=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),x,y)]);
        assert_eq!(a.data(),expected);
        let b=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),x-7.0,y+3.0)]);
        assert_eq!(a.data(),b.data());
        let c=draw(vec![instance(rectangle(0.0,0.0,1.0,1.0),x,y)]);
        assert_eq!(a.data(),c.data());
    }
    let a=instance(rectangle(0.0,0.0,1.0,1.0),-1.0/64.0,0.0);
    assert_eq!(a.phase().0,63);assert_eq!(a.offset().x(),-1);
}
#[test]
fn raster_limits() {
    let g=group(vec![instance(rectangle(0.0,0.0,2.0,2.0),0.0,0.0)]);
    for limits in [Limits::default().with_transient_bytes(1).unwrap(),Limits::default().with_output_bytes(1).unwrap(),Limits::default().with_output_pixels(1).unwrap(),Limits::default().with_max_dimension(1).unwrap(),Limits::default().with_path_segments_per_frame(1).unwrap()] {
        assert!(rasterize(&mut RasterContext::with_limits(limits).unwrap(),&g).is_err());
    }
    let mut ctx=RasterContext::new().unwrap();
    let bounds=PixelRect::new(0,0,2,2).unwrap();
    assert!(ctx.rasterize_path(&[],bounds,usize::MAX).is_err());
    let invalid=[PathOp::LineTo(DevicePoint::new(Q6::from_ticks(0),Q6::from_ticks(0)))];
    assert!(ctx.rasterize_path(&invalid,bounds,2).is_err());
    assert!(Mask::new(bounds,usize::MAX,Arc::from([])).is_err());
    assert!(Mask::new(bounds,1,Arc::from([0;2])).is_err());
    assert!(Mask::new(PixelRect::new(0,0,1,1).unwrap(),2,Arc::from([255,1])).is_err());
    assert!(CoverageGroup::new(vec![],CoveragePass::Shadow,Profile::CoreV1,&Limits::default()).is_err());
    assert!(CoverageGroup::new(vec![],CoveragePass::Fill,Profile::LegacyXyV1,&Limits::default()).is_err());
}
