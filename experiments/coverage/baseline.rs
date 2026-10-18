// Independent R002 experiment driver; production crates are unchanged.
#![forbid(unsafe_code)]
use std::{fs, path::Path, sync::Arc, time::Instant};
use taiyou_render::{geometry::*, raster::*};
use taiyou_types::*;

fn f(v: f64) -> FiniteF64 { FiniteF64::new(v).unwrap() }
fn p(x: f64, y: f64) -> FontPoint { FontPoint::new(x, -y).unwrap() }
fn polygon(points: &[(f64, f64)]) -> Vec<PathOp<FontPoint>> {
    let mut ops = vec![PathOp::MoveTo(p(points[0].0, points[0].1))];
    ops.extend(points[1..].iter().map(|&(x,y)| PathOp::LineTo(p(x,y))));
    ops.push(PathOp::Close);
    ops
}
fn rect() -> Vec<PathOp<FontPoint>> {
    polygon(&[(0.,0.), (8.,0.), (8.,8.), (0.,8.)])
}
fn instance(ops: Vec<PathOp<FontPoint>>, x: f64, y: f64, angle: f64, sx: f64, sy: f64) -> GeometryInstance {
    let outline = Arc::new(Outline::new(ops, &Limits::default()).unwrap());
    let map = GeometryMap::new(
        (f(sx), f(sy)), ScriptPoint::new(f(0.), f(0.)),
        ScriptPoint::new(f(x), f(y)), None, f(angle),
        DeviceMapping::new(Size::new(64,64), PixelRect::new(0,0,64,64).unwrap()).unwrap()
    ).unwrap();
    map_outline(outline, map, &Limits::default()).unwrap()
}
fn main() {
    let out = std::env::args().nth(1).unwrap();
    let out = Path::new(&out);
    fs::create_dir_all(out).unwrap();
    let mut cases = Vec::new();
    for (id,x,y) in [
        ("straight",0.,0.), ("phase-quarter",0.25,0.), ("phase-1of64",1./64.,0.),
        ("phase-7of64",7./64.,0.), ("negative",-0.25,-1.5),
        ("offscreen",-20.25,-12.5)
    ] {
        cases.push((id, vec![instance(rect(),x,y,0.,1.,1.)]));
    }
    cases.push(("acute", vec![instance(polygon(&[(0.,8.),(4.,0.),(4.25,8.)]),0.,0.,0.,1.,1.)]));
    cases.push(("self-cross", vec![instance(polygon(&[(0.,0.),(8.,8.),(0.,8.),(8.,0.)]),0.,0.,0.,1.,1.)]));
    cases.push(("quadratic", vec![instance(vec![PathOp::MoveTo(p(0.,8.)),PathOp::QuadTo(p(4.,-4.),p(8.,8.)),PathOp::Close],0.,0.,0.,1.,1.)]));
    cases.push(("cubic", vec![instance(vec![PathOp::MoveTo(p(0.,8.)),PathOp::CubicTo(p(0.,-4.),p(8.,-4.),p(8.,8.)),PathOp::Close],0.,0.,0.,1.,1.)]));
    let mut hole = rect();
    hole.extend(polygon(&[(2.,2.),(2.,6.),(6.,6.),(6.,2.)]));
    cases.push(("hole",vec![instance(hole,0.,0.,0.,1.,1.)]));
    cases.push(("rotated-scaled",vec![instance(rect(),0.25,0.5,30.,2.,0.5)]));
    cases.push(("overlap-combined",vec![instance(rect(),0.,0.,0.,1.,1.),instance(rect(),4.,0.,0.,1.,1.)]));
    cases.push(("overlap-a",vec![instance(rect(),0.,0.,0.,1.,1.)]));
    cases.push(("overlap-b",vec![instance(rect(),4.,0.,0.,1.,1.)]));
    let mut ctx = RasterContext::new().unwrap();
    fs::write(out.join("backend.txt"),ctx.backend()).unwrap();
    let mut table = String::from("case\tx\ty\twidth\theight\tstride\tmask_bytes\tns_samples\n");
    for (id, instances) in cases {
        let group = CoverageGroup::new(instances, CoveragePass::Fill, Profile::CoreV1, &Limits::default()).unwrap();
        fs::write(out.join(format!("{id}.geometry.txt")),format!("{:#?}",group.instances())).unwrap();
        let first = rasterize(&mut ctx, &group).unwrap();
        let second = rasterize(&mut ctx, &group).unwrap();
        assert_eq!(first.bounds(),second.bounds(),"repeat bounds mismatch: {id}");
        assert_eq!(first.stride(),second.stride(),"repeat stride mismatch: {id}");
        assert_eq!(first.data(),second.data(),"repeat bytes mismatch: {id}");
        // Warmed in-process smoke measurement, not a speedup claim.
        let mut samples = Vec::new();
        for _ in 0..30 {
            let start = Instant::now();
            for _ in 0..20 { std::hint::black_box(rasterize(&mut ctx, &group).unwrap()); }
            samples.push((start.elapsed().as_nanos()/20).to_string());
        }
        fs::write(out.join(format!("{id}.a8")),first.data()).unwrap();
        let b=first.bounds();
        table.push_str(&format!("{id}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",b.x0(),b.y0(),b.width(),b.height(),first.stride(),first.data().len(),samples.join(",")));
    }
    fs::write(out.join("samples.tsv"),table).unwrap();
}
