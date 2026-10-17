//! C5 design-unit contours -> checked Q6 paths. No rasterization or clipping.
use std::sync::Arc;
use taiyou_types::*;

fn finite(v:f64)->Result<FiniteF64,RenderError> { FiniteF64::new(v).map_err(|_|geometry_limit()) }
fn q6(v:f64)->Result<Q6,RenderError> { Q6::from_pixels(finite(v)?).map_err(|_|geometry_limit()) }

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceMapping { play_res: Size, content: PixelRect }
impl DeviceMapping {
    pub fn new(play_res:Size,content:PixelRect)->Result<Self,RenderError> {
        if play_res.is_empty() || content.width()==0 || content.height()==0 { return Err(RenderError::new(ErrorCode::InvalidArgument,"empty geometry mapping")); }
        Ok(Self{play_res,content})
    }
    pub const fn play_res(self)->Size { self.play_res }
    pub const fn content(self)->PixelRect { self.content }
}
/// A complete immutable map. `position` already includes HarfBuzz offsets;
/// `scale` converts design units to script units, including local ASS scale once.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeometryMap {
    scale:(FiniteF64,FiniteF64), position:ScriptPoint, anchor:ScriptPoint,
    origin:ScriptPoint, rotation:FiniteF64, device:DeviceMapping,
}
impl GeometryMap {
    pub fn new(scale:(FiniteF64,FiniteF64),position:ScriptPoint,anchor:ScriptPoint,origin:Option<ScriptPoint>,rotation:FiniteF64,device:DeviceMapping)->Result<Self,RenderError> {
        if scale.0.get()<0.0 || scale.1.get()<0.0 { return Err(RenderError::new(ErrorCode::InvalidArgument,"negative outline scale")); }
        Ok(Self{scale,position,anchor,origin:origin.unwrap_or(anchor),rotation,device})
    }
    pub fn point(self,p:FontPoint)->Result<DevicePoint,RenderError> {
        // Font y-up conversion occurs exactly once, before rotation and viewport.
        let x=p.x().get()*self.scale.0.get()+self.position.x().get()+self.anchor.x().get()-self.origin.x().get();
        let y=-p.y().get()*self.scale.1.get()+self.position.y().get()+self.anchor.y().get()-self.origin.y().get();
        finite(x)?; finite(y)?;
        let angle=self.rotation.get().rem_euclid(360.0);
        // Exact cardinal rotations avoid artificial one-tick errors at right angles.
        let (sin,cos)=match angle {0.0=>(0.0,1.0),90.0=>(1.0,0.0),180.0=>(0.0,-1.0),270.0=>(-1.0,0.0),_=>angle.to_radians().sin_cos()};
        let (x,y)=(cos*x+sin*y+self.origin.x().get(),-sin*x+cos*y+self.origin.y().get());
        Ok(DevicePoint::new(
            q6(x*(f64::from(self.device.content.width())/f64::from(self.device.play_res.width()))+f64::from(self.device.content.x0()))?,
            q6(y*(f64::from(self.device.content.height())/f64::from(self.device.play_res.height()))+f64::from(self.device.content.y0()))?,
        ))
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceBounds { min:DevicePoint,max:DevicePoint }
impl DeviceBounds {
    pub const fn min(self)->DevicePoint { self.min }
    pub const fn max(self)->DevicePoint { self.max }
}
/// Local Q6 commands plus whole-pixel offset reconstruct the absolute mapped path.
/// Bounds are conservative control bounds, distinct from layout ink/collision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryInstance {
    outline:Arc<Outline>, mapping:GeometryMap, ops:Arc<[PathOp<DevicePoint>]>,
    offset:PixelOffset, phase:(u8,u8), bounds:Option<DeviceBounds>,
}
impl GeometryInstance {
    pub fn outline(&self)->&Arc<Outline> { &self.outline }
    pub const fn mapping(&self)->GeometryMap { self.mapping }
    pub fn ops(&self)->&[PathOp<DevicePoint>] { &self.ops }
    pub const fn offset(&self)->PixelOffset { self.offset }
    pub const fn phase(&self)->(u8,u8) { self.phase }
    pub const fn bounds(&self)->Option<DeviceBounds> { self.bounds }
}
pub fn map_outline(outline:Arc<Outline>,mapping:GeometryMap,limits:&Limits)->Result<GeometryInstance,RenderError> {
    check_path_budget(outline.ops().len(),limits)?;
    let zero=mapping.point(FontPoint::ZERO)?;
    let (ox,px)=zero.x().split(); let (oy,py)=zero.y().split();
    let mut ops=Vec::new();
    ops.try_reserve_exact(outline.ops().len()).map_err(|_|RenderError::new(ErrorCode::AllocationFailed,"device path allocation failed"))?;
    let mut bounds:Option<DeviceBounds>=None;
    for &op in outline.ops() {
        ops.push(op.try_map(|p| {
            let p=mapping.point(p)?;
            bounds=Some(match bounds {
                None=>DeviceBounds{min:p,max:p},Some(b)=>DeviceBounds {
                    min:DevicePoint::new(p.x().min(b.min.x()),p.y().min(b.min.y())),
                    max:DevicePoint::new(p.x().max(b.max.x()),p.y().max(b.max.y())),
                }
            });
            let local=|q:Q6,offset:i32| i32::try_from(i64::from(q.ticks())-i64::from(offset)*64).map(Q6::from_ticks).map_err(|_|geometry_limit());
            Ok(DevicePoint::new(local(p.x(),ox)?,local(p.y(),oy)?))
        })?);
    }
    if let Some(b)=bounds {
        let cap=i64::from(limits.max_dimension())*64;
        if i64::from(b.max.x().ticks())-i64::from(b.min.x().ticks())>cap || i64::from(b.max.y().ticks())-i64::from(b.min.y().ticks())>cap { return Err(geometry_limit()); }
    }
    Ok(GeometryInstance{outline,mapping,ops:ops.into(),offset:PixelOffset::new(ox,oy),phase:(px as u8,py as u8),bounds})
}

/// Placement is explicit: layout is local and does not infer viewport margins.
/// An optional host origin is absolute script space. Current Core parsing has no
/// org operation; None uses the supplied alignment anchor, never invented offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GeometryPlacement { pub anchor:ScriptPoint, pub origin:Option<ScriptPoint>, pub device:DeviceMapping }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryGlyph { pub outline:GlyphOutline, pub geometry:GeometryInstance, pub cluster:DecodedSpan, pub association:usize }
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeometryEvent { layout:Arc<LocalLayout>,glyphs:Arc<[GeometryGlyph]> }
impl GeometryEvent {
    pub fn layout(&self)->&LocalLayout { &self.layout }
    pub fn glyphs(&self)->&[GeometryGlyph] { &self.glyphs }
}
#[cfg(feature="native")]
pub fn geometry(context:&mut taiyou_font::FontContext,catalog:&taiyou_font::ValidatedFontCatalog,layout:Arc<LocalLayout>,placement:GeometryPlacement,limits:&Limits)->Result<GeometryEvent,RenderError> {
    let fail=|e:FontError| RenderError::new(e.code(),e.detail());
    let count=layout.glyphs().len();
    let mut charge=(count as u64).checked_mul(3*std::mem::size_of::<GeometryGlyph>() as u64).ok_or_else(geometry_limit)?;
    if count as u64>limits.glyphs_per_event() || charge>limits.transient_bytes() { return Err(geometry_limit()); }
    let mut glyphs=Vec::new();
    glyphs.try_reserve_exact(count).map_err(|_|RenderError::new(ErrorCode::AllocationFailed,"geometry glyph allocation failed"))?;
    let mut total=0usize;
    for positioned in layout.glyphs() {
        let run=&layout.shaped().runs()[positioned.run_index()];
        let glyph=&run.glyphs()[positioned.glyph_index()];
        let state=&layout.shaped().evaluated().runs()[positioned.association_index()];
        let blob=catalog.faces().iter().find(|f| f.blob().id()==glyph.font()).ok_or_else(||RenderError::new(ErrorCode::FontUnavailable,"outline font absent from validated catalog"))?.blob();
        let outline=context.outline(blob,glyph.glyph(),limits).map_err(fail)?;
        total=total.checked_add(outline.outline().ops().len()).ok_or_else(geometry_limit)?;
        check_path_budget(total,limits)?;
        charge=charge.checked_add((outline.outline().ops().len() as u64).checked_mul(4*(std::mem::size_of::<PathOp<FontPoint>>()+std::mem::size_of::<PathOp<DevicePoint>>()) as u64).ok_or_else(geometry_limit)?).ok_or_else(geometry_limit)?;
        if charge.checked_add(4096).is_none_or(|n| n>limits.transient_bytes()) { return Err(geometry_limit()); }
        let size=state.shaping().font_size().get()/f64::from(outline.units_per_em());
        let additional=layout.local_scale();
        let mapping=GeometryMap::new((finite(size*state.geometry().scale_x().get()*additional.0.get())?,finite(size*state.geometry().scale_y().get()*additional.1.get())?),positioned.position(),placement.anchor,placement.origin.or(layout.shaped().placement().origin()),state.geometry().rotation_z(),placement.device)?;
        let mapped=map_outline(outline.outline().clone(),mapping,limits)?;
        glyphs.push(GeometryGlyph{outline,geometry:mapped,cluster:positioned.cluster().clone(),association:positioned.association_index()});
    }
    Ok(GeometryEvent{layout,glyphs:glyphs.into()})
}
