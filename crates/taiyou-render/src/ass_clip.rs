//! Event clip slots, resolved after layout and effects, before composition.
use std::sync::Arc;
use taiyou_types::*;
use crate::geometry::{DeviceMapping,GeometryMap};
use crate::raster::RasterContext;
fn limit()->RenderError {RenderError::new(ErrorCode::LimitExceeded,"ASS clip resource/coordinate limit")}
pub(crate) fn resolve(spec:&AssClip,device:DeviceMapping,limits:&Limits)->Result<(Arc<ResolvedClip>,Option<Arc<Mask>>,u64),RenderError> {
    let sx=f64::from(device.content().width())/f64::from(device.play_res().width());
    let sy=f64::from(device.content().height())/f64::from(device.play_res().height());
    let mut result=Arc::new(ResolvedClip::None);
    if !spec.rectangles.is_empty() {
        let mut current=[0.0,0.0,f64::from(device.content().width()),f64::from(device.content().height())];
        let mut inverse=false;
        for step in spec.rectangles.iter() {
            for i in 0..4 {
                let dst=f64::from(step.coords[i])*if i%2==0 {sx}else{sy};
                let value=if (dst-current[i]).abs()<0.0001 {dst}else{(1.0-step.progress.get())*current[i]+step.progress.get()*dst};
                if !value.is_finite() || value<f64::from(i32::MIN)||value>f64::from(i32::MAX) {return Err(limit())}
                current[i]=value.trunc();
            }
            inverse=step.inverse;
        }
        let mut c=[0;4];
        for i in 0..4 {c[i]=(current[i] as i32).checked_add(if i%2==0 {device.content().x0()}else{device.content().y0()}).ok_or_else(limit)?;}
        // Empty/inverted rectangles have no inside; do not normalize endpoints.
        c[2]=c[2].max(c[0]);c[3]=c[3].max(c[1]);
        result=Arc::new(ResolvedClip::Rectangle {bounds:PixelRect::new(c[0],c[1],c[2],c[3]).map_err(|_|limit())?,inverse});
    }
    let mut retained=None;
    let mut segments=0;
    if let Some((drawing,inverse))=&spec.vector {
        segments=drawing.outline.ops().len() as u64;
        check_path_budget(drawing.outline.ops().len(),limits)?;
        let charge=segments.checked_mul(4*std::mem::size_of::<PathOp<DevicePoint>>() as u64).ok_or_else(limit)?;
        let remaining=limits.transient_bytes().checked_sub(charge).filter(|n|*n>0).ok_or_else(||limit().with_resource_failure(ResourceFailure::TransientMemory))?;
        let native_limits=limits.clone().with_transient_bytes(remaining).map_err(|_|limit())?;
        let zero=ScriptPoint::new(FiniteF64::ZERO,FiniteF64::ZERO);
        let mapping=GeometryMap::new((FiniteF64::ONE,FiniteF64::ONE),zero,zero,None,FiniteF64::ZERO,device)?;
        let mut path=Vec::new();
        path.try_reserve_exact(drawing.outline.ops().len()).map_err(|_|RenderError::new(ErrorCode::AllocationFailed,"clip path allocation failed"))?;
        for op in drawing.outline.ops() {path.push(op.try_map(|p|mapping.point(p))?);}
        let bounds=crate::stroke::bounds_for_path(&path)?;
        let mask=if bounds.is_empty() {Arc::new(Mask::new(PixelRect::new(0,0,0,0).map_err(|_|limit())?,0,Arc::from([]))?)}else{
            let mut raster=RasterContext::with_limits(native_limits)?;
            raster.rasterize_path(&path,bounds,usize::try_from(bounds.width()).map_err(|_|limit())?)?
        };
        let vector=Arc::new(ResolvedClip::MaskCoverage {mask:mask.clone(),offset:PixelOffset::default(),inverse:*inverse});
        result=Arc::new(ResolvedClip::Intersection(result,vector));
        retained=Some(mask);
    }
    Ok((result,retained,segments))
}
