//! Bounded renderer-owned retention of immutable, paint-independent masks.
#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub bypasses: u64,
    pub entries: usize,
    pub retained_bytes: u64,
    pub peak_bytes: u64,
}

#[cfg(feature = "native")]
mod masks {
    use super::CacheStats;
    use crate::{raster::*, timing::StageTimings};
    use std::{mem::size_of, sync::Arc};
    use taiyou_types::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Kind {
        Body,
        Border(Q6, Q6),
    }
    struct FilterKey {
        spec: crate::effects::BlurSpec,
        data: Vec<u8>,
        width: u32,
        height: u32,
        stride: usize,
        xy_units: bool,
    }
    struct Key {
        schema_revision: u32,
        profile: Profile,
        coverage_revision: u32,
        // Full ordered Q6 path, with only whole-pixel translation factored out.
        ops: Vec<PathOp<DevicePoint>>,
        kind: Kind,
        limits: Limits,
        filter: Option<FilterKey>,
    }
    struct Entry {
        key: Key,
        mask: Arc<Mask>,
        origin: PixelOffset,
        segments: u64,
        required_peak: u64,
        required_work: u64,
        touched: u64,
        charge: u64,
    }
    pub(crate) struct MaskCache {
        filter_retention: bool,
        entries: Vec<Entry>,
        limit: u64,
        used: u64,
        ordinal: u64,
        stats: CacheStats,
    }
    fn origin(ops: &[PathOp<DevicePoint>]) -> PixelOffset {
        let mut minimum: Option<(i32, i32)> = None;
        for op in ops {
            // All path values are already validated Q6; this visit cannot fail.
            let _ = op.try_map(|p| {
                let (x, y) = (p.x().ticks(), p.y().ticks());
                minimum = Some(minimum.map_or((x, y), |(a, b)| (a.min(x), b.min(y))));
                Ok(p)
            });
        }
        minimum.map_or(PixelOffset::default(), |(x, y)| {
            PixelOffset::new(x.div_euclid(64), y.div_euclid(64))
        })
    }
    fn local(op: PathOp<DevicePoint>, origin: PixelOffset) -> Option<PathOp<DevicePoint>> {
        op.try_map(|p| {
            let q = |n: Q6, offset: i32| {
                i32::try_from(i64::from(n.ticks()) - i64::from(offset) * 64)
                    .map(Q6::from_ticks)
                    .map_err(|_| geometry_limit())
            };
            Ok(DevicePoint::new(
                q(p.x(), origin.x())?,
                q(p.y(), origin.y())?,
            ))
        })
        .ok()
    }
    fn relocated(mask: &Arc<Mask>, old: PixelOffset, new: PixelOffset) -> Option<Arc<Mask>> {
        if old == new || mask.bounds().is_empty() {
            return Some(mask.clone());
        }
        let b = mask.bounds();
        let x = |n: i32| i32::try_from(i64::from(n) + i64::from(new.x()) - i64::from(old.x())).ok();
        let y = |n: i32| i32::try_from(i64::from(n) + i64::from(new.y()) - i64::from(old.y())).ok();
        let bounds = PixelRect::new(x(b.x0())?, y(b.y0())?, x(b.x1())?, y(b.y1())?).ok()?;
        // A local coverage copy, not an RGBA frame copy. No native work or
        // resampling. Failure bypasses reuse and runs the ordinary producer.
        let mut data = Vec::new();
        data.try_reserve_exact(mask.data().len()).ok()?;
        data.extend_from_slice(mask.data());
        Some(Arc::new(
            mask.with_data(bounds, mask.stride(), data.into()).ok()?,
        ))
    }
    impl MaskCache {
        pub(crate) fn new(limit: u64) -> Self {
            let mut entries = Vec::new();
            let count = (limit / (size_of::<Entry>() as u64 * 4)).min(128) as usize;
            if entries.try_reserve_exact(count).is_err() {
                entries = Vec::new();
            }
            if entries.capacity() as u64 * size_of::<Entry>() as u64 > limit {
                entries = Vec::new();
            }
            let used = entries.capacity() as u64 * size_of::<Entry>() as u64;
            Self {
                filter_retention: true,
                entries,
                limit,
                used,
                ordinal: 0,
                stats: CacheStats::default(),
            }
        }
        pub(crate) fn begin_frame(&mut self) {
            self.stats = CacheStats {
                peak_bytes: self.used,
                ..Default::default()
            };
        }
        pub(crate) fn with_filter_retention(mut self, enabled: bool) -> Self {
            self.filter_retention=enabled;
            self
        }
        pub(crate) fn stats(&self) -> CacheStats {
            CacheStats {
                entries: self.entries.len(),
                retained_bytes: self.used,
                ..self.stats
            }
        }
        fn make_room(&mut self, charge: u64) -> bool {
            // Pinned allocations stay indexed and charged until their consumers
            // finish; removing an index alone is not a memory release.
            let index_bytes=self.entries.capacity() as u64 * size_of::<Entry>() as u64;
            if self.entries.capacity()==0 || charge>self.limit.saturating_sub(index_bytes) {
                self.stats.bypasses+=1;
                return false;
            }
            while self.used.saturating_add(charge)>self.limit || self.entries.len()==self.entries.capacity() {
                let victim=self.entries.iter().enumerate().filter(|(_,e)|Arc::strong_count(&e.mask)==1).min_by_key(|(i,e)|(e.touched,*i)).map(|(i,_)|i);
                let Some(index)=victim else { self.stats.bypasses+=1; return false; };
                let removed=self.entries.remove(index);
                self.used-=removed.charge;
                self.stats.evictions+=1;
            }
            true
        }
        pub(crate) fn filter(&mut self, mask:&Arc<Mask>,spec:&crate::effects::BlurSpec,limits:&Limits) -> Result<Arc<Mask>,RenderError> {
            if !spec.active() || mask.bounds().is_empty() {
                return Ok(mask.clone());
            }
            if self.entries.capacity()==0 || !self.filter_retention {
                self.stats.misses+=1;
                return crate::effects::apply_effects(mask,spec,limits);
            }
            // Run the original combined scratch/extent preflight on hits too.
            crate::effects::preflight_effects(mask,spec,limits)?;
            let key_limits=limits.clone().with_cache_bytes(0).with_transient_bytes(u64::MAX).map_err(|_|geometry_limit())?;
            let bounds=mask.bounds();
            let origin=PixelOffset::new(bounds.x0(),bounds.y0());
            self.ordinal=self.ordinal.saturating_add(1);
            for entry in &mut self.entries {
                let Some(filter)=entry.key.filter.as_ref() else { continue; };
                if entry.key.schema_revision==1 && entry.key.profile==Profile::CoreV1 && entry.key.coverage_revision==1
                    && entry.key.limits==key_limits && filter.spec==*spec
                    && filter.width==bounds.width() && filter.height==bounds.height()
                    && filter.stride==mask.stride() && filter.xy_units==mask.is_xy_coverage()
                    && filter.data==mask.data()
                {
                    if let Some(result)=relocated(&entry.mask,entry.origin,origin) {
                        self.stats.hits+=1;
                        entry.touched=self.ordinal;
                        return Ok(result);
                    }
                }
            }
            self.stats.misses+=1;
            let result=crate::effects::apply_effects(mask,spec,limits)?;
            if Arc::ptr_eq(mask,&result) {
                self.stats.bypasses+=1;
                return Ok(result);
            }
            let charge=(mask.data().len() as u64).saturating_add(result.data().len() as u64).saturating_add(size_of::<Mask>() as u64+32);
            if !self.make_room(charge) { return Ok(result); }
            let mut data=Vec::new();
            if data.try_reserve_exact(mask.data().len()).is_err() { self.stats.bypasses+=1; return Ok(result); }
            data.extend_from_slice(mask.data());
            let actual_charge=charge.saturating_add((data.capacity()-data.len()) as u64);
            if !self.make_room(actual_charge) { return Ok(result); }
            self.used+=actual_charge;
            self.stats.peak_bytes=self.stats.peak_bytes.max(self.used);
            self.entries.push(Entry {
                key:Key { schema_revision:1,profile:Profile::CoreV1,coverage_revision:1,ops:Vec::new(),kind:Kind::Body,limits:key_limits,
                    filter:Some(FilterKey { spec:spec.clone(),data,width:bounds.width(),height:bounds.height(),stride:mask.stride(),xy_units:mask.is_xy_coverage() }) },
                mask:result.clone(),origin,segments:0,required_peak:0,required_work:0,touched:self.ordinal,charge:actual_charge,
            });
            Ok(result)
        }
        pub(crate) fn fill(
            &mut self,
            ctx: &mut RasterContext,
            group: &CoverageGroup,
            timings: &mut StageTimings,
        ) -> Result<Arc<Mask>, RenderError> {
            let remaining = ctx.limits().path_segments_per_frame();
            self.render(
                ctx,
                group,
                Kind::Body,
                Q6::from_ticks(0),
                Q6::from_ticks(0),
                remaining,
                timings,
            )
            .map(|v| v.0)
        }
        pub(crate) fn translated(
            &mut self,
            ctx: &mut RasterContext,
            group: &CoverageGroup,
            dx: Q6,
            dy: Q6,
            timings: &mut StageTimings,
        ) -> Result<Arc<Mask>, RenderError> {
            let remaining = ctx.limits().path_segments_per_frame();
            self.render(ctx, group, Kind::Body, dx, dy, remaining, timings)
                .map(|v| v.0)
        }
        #[allow(clippy::too_many_arguments)]
        pub(crate) fn border(
            &mut self,
            ctx: &mut RasterContext,
            group: &CoverageGroup,
            rx: Q6,
            ry: Q6,
            dx: Q6,
            dy: Q6,
            remaining: u64,
            timings: &mut StageTimings,
        ) -> Result<(Arc<Mask>, u64), RenderError> {
            // Projected native strokes have an additional pre-transform path.
            // Keep their existing producer until that dependency is profiled.
            if group
                .instances()
                .first()
                .is_some_and(|g| g.mapping().transforms_border())
            {
                self.stats.bypasses += 1;
                return crate::stroke::rasterize_expanded_axes_xy_timed(
                    ctx, group, rx, ry, dx, dy, remaining, timings,
                );
            }
            self.render(ctx, group, Kind::Border(rx, ry), dx, dy, remaining, timings)
        }
        #[allow(clippy::too_many_arguments)]
        fn render(
            &mut self,
            ctx: &mut RasterContext,
            group: &CoverageGroup,
            kind: Kind,
            dx: Q6,
            dy: Q6,
            remaining: u64,
            timings: &mut StageTimings,
        ) -> Result<(Arc<Mask>, u64), RenderError> {
            if group.pass() != CoveragePass::Fill || group.policy().profile() != Profile::CoreV1 {
                return Err(RenderError::new(
                    ErrorCode::UnsupportedFeature,
                    "CoreV1 mask cache requires a fill geometry group",
                ));
            }
            if self.entries.capacity() == 0 {
                self.stats.misses += 1;
                self.stats.bypasses += 1;
                return match kind {
                    Kind::Body => crate::stroke::rasterize_translated_xy_timed(ctx, group, dx, dy, timings).map(|mask| (mask, 0)),
                    Kind::Border(rx, ry) => crate::stroke::rasterize_expanded_axes_xy_timed(ctx, group, rx, ry, dx, dy, remaining, timings),
                };
            }
            // Preserve the original path validation/allocation and reservation.
            let ops = coverage_path(group, dx, dy, ctx.limits())?;
            let path_bytes = (ops.len() as u64)
                .checked_mul(3 * size_of::<PathOp<DevicePoint>>() as u64)
                .ok_or_else(geometry_limit)?;
            let available = ctx
                .limits()
                .transient_bytes()
                .checked_sub(path_bytes)
                .filter(|&n| n > 0)
                .ok_or_else(|| RenderError::transient_limit("mask path reservation exceeded"))?;
            let limits = ctx
                .limits()
                .clone()
                .with_transient_bytes(available)
                .map_err(|_| geometry_limit())?;
            let key_limits = limits
                .clone()
                .with_cache_bytes(0)
                .with_transient_bytes(u64::MAX)
                .map_err(|_| geometry_limit())?;
            let origin = origin(&ops);
            self.ordinal = self.ordinal.saturating_add(1);
            if self.limit != 0 {
                for entry in &mut self.entries {
                    if entry.key.filter.is_none()
                        && entry.key.schema_revision == 1
                        && entry.key.profile == group.policy().profile()
                        && entry.key.coverage_revision == group.policy().revision()
                        && entry.key.kind == kind
                        && entry.key.limits == key_limits
                        && entry.required_peak <= available
                        && entry.required_work <= remaining
                        && entry.key.ops.len() == ops.len()
                        && entry
                            .key
                            .ops
                            .iter()
                            .zip(&ops)
                            .all(|(a, b)| Some(*a) == local(*b, origin))
                    {
                        if let Some(mask) = relocated(&entry.mask, entry.origin, origin) {
                            entry.touched = self.ordinal;
                            self.stats.hits += 1;
                            return Ok((mask, entry.segments));
                        }
                    }
                }
            }
            self.stats.misses += 1;
            let mut observed = StageTimings {
                enabled: timings.enabled,
                ..Default::default()
            };
            let result = match kind {
                Kind::Body => crate::axis_widen::rasterize_body_timed(&ops, &limits, &mut observed)
                    .map(|mask| (mask, 0)),
                Kind::Border(rx, ry) => crate::axis_widen::rasterize_axis_xy_timed(
                    &ops,
                    rx,
                    ry,
                    remaining,
                    &limits,
                    &mut observed,
                ),
            };
            timings.merge_raster(&observed);
            let (mask, segments) = result?;
            // Errors are never retained. Admission failure only changes cost.
            let charge = (ops.len() as u64)
                .saturating_mul(size_of::<PathOp<DevicePoint>>() as u64)
                .saturating_add(mask.data().len() as u64)
                .saturating_add(size_of::<Mask>() as u64 + 32);
            if !self.make_room(charge) { return Ok((mask, segments)); }
            let mut canonical = Vec::new();
            if canonical.try_reserve_exact(ops.len()).is_err() {
                self.stats.bypasses += 1;
                return Ok((mask, segments));
            }
            for &op in &ops {
                let Some(op) = local(op, origin) else {
                    self.stats.bypasses += 1;
                    return Ok((mask, segments));
                };
                canonical.push(op);
            }
            let actual_charge = charge.saturating_add(
                (canonical.capacity() - canonical.len()) as u64
                    * size_of::<PathOp<DevicePoint>>() as u64,
            );
            if self.used.saturating_add(actual_charge) > self.limit {
                self.stats.bypasses += 1;
                return Ok((mask, segments));
            }
            self.used += actual_charge;
            self.stats.peak_bytes = self.stats.peak_bytes.max(self.used);
            self.entries.push(Entry {
                key: Key {
                    schema_revision: 1,
                    profile: group.policy().profile(),
                    coverage_revision: group.policy().revision(),
                    ops: canonical,
                    kind,
                    limits: key_limits,
                    filter: None,
                },
                mask: mask.clone(),
                origin,
                segments,
                required_peak: observed.raster_peak_bytes,
                required_work: observed.raster_required_work,
                touched: self.ordinal,
                charge: actual_charge,
            });
            Ok((mask, segments))
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::geometry::*;
        fn group(size: f64, at: f64) -> CoverageGroup {
            let f = |n| FiniteF64::new(n).unwrap();
            let p = |x: f64, y: f64| FontPoint::new(x, -y).unwrap();
            let outline = Arc::new(
                Outline::new(
                    vec![
                        PathOp::MoveTo(p(0.0, 0.0)),
                        PathOp::LineTo(p(size, 0.0)),
                        PathOp::LineTo(p(size, size)),
                        PathOp::LineTo(p(0.0, size)),
                        PathOp::Close,
                    ],
                    &Limits::default(),
                )
                .unwrap(),
            );
            let mapping = GeometryMap::new(
                (f(1.0), f(1.0)),
                ScriptPoint::new(f(0.0), f(0.0)),
                ScriptPoint::new(f(at), f(at)),
                None,
                f(0.0),
                DeviceMapping::new(Size::new(1, 1), PixelRect::new(0, 0, 1, 1).unwrap()).unwrap(),
            )
            .unwrap();
            CoverageGroup::new(
                vec![map_outline(outline, mapping, &Limits::default()).unwrap()],
                CoveragePass::Fill,
                Profile::CoreV1,
                &Limits::default(),
            )
            .unwrap()
        }
        #[test]
        fn hits_preserve_original_memory_and_work_failures() {
            let mut cache = MaskCache::new(64 * 1024);
            let limits = Limits::default();
            let g = group(40.0, 0.0);
            let q = Q6::from_ticks(8 * 64);
            drop(
                cache
                    .border(
                        &mut RasterContext::with_limits(limits.clone()).unwrap(),
                        &g,
                        q,
                        q,
                        Q6::from_ticks(0),
                        Q6::from_ticks(0),
                        limits.path_segments_per_frame(),
                        &mut Default::default(),
                    )
                    .unwrap(),
            );
            assert_eq!(cache.entries.len(), 1);
            let required_peak = cache.entries[0].required_peak;
            let required_work = cache.entries[0].required_work;
            assert!(required_peak > 1);
            assert!(required_work > 1);
            for (limits, remaining) in [
                (
                    limits
                        .clone()
                        .with_transient_bytes(required_peak - 1)
                        .unwrap(),
                    limits.path_segments_per_frame(),
                ),
                (limits.clone(), required_work - 1),
            ] {
                let mut uncached = MaskCache::new(0);
                let a = uncached
                    .border(
                        &mut RasterContext::with_limits(limits.clone()).unwrap(),
                        &g,
                        q,
                        q,
                        Q6::from_ticks(0),
                        Q6::from_ticks(0),
                        remaining,
                        &mut Default::default(),
                    )
                    .unwrap_err();
                let b = cache
                    .border(
                        &mut RasterContext::with_limits(limits).unwrap(),
                        &g,
                        q,
                        q,
                        Q6::from_ticks(0),
                        Q6::from_ticks(0),
                        remaining,
                        &mut Default::default(),
                    )
                    .unwrap_err();
                assert_eq!(a.code(), b.code());
                assert_eq!(a.resource_failure(), b.resource_failure());
            }
            assert_eq!(cache.stats().hits, 0);
        }
        #[test]
        fn pinned_entries_stay_charged_oversize_bypasses_and_eviction_reclaims() {
            let mut cache = MaskCache::new(4096);
            let mut ctx = RasterContext::new().unwrap();
            let pinned = cache
                .fill(&mut ctx, &group(40.0, 0.0), &mut Default::default())
                .unwrap();
            assert_eq!(cache.entries.len(), 1);
            let used = cache.stats().retained_bytes;
            drop(
                cache
                    .fill(&mut ctx, &group(200.0, 0.0), &mut Default::default())
                    .unwrap(),
            );
            assert_eq!(cache.stats().retained_bytes, used);
            assert_eq!(cache.stats().evictions, 0);
            drop(
                cache
                    .fill(&mut ctx, &group(41.0, 0.0), &mut Default::default())
                    .unwrap(),
            );
            assert_eq!(cache.stats().retained_bytes, used);
            assert_eq!(cache.stats().evictions, 0);
            drop(pinned);
            drop(
                cache
                    .fill(&mut ctx, &group(41.0, 0.0), &mut Default::default())
                    .unwrap(),
            );
            assert_eq!(cache.stats().evictions, 1);
            assert!(cache.stats().retained_bytes <= 4096);
        }
        #[test]
        fn full_path_equality_and_integer_relocation_preserve_phase() {
            let mut cache = MaskCache::new(64 * 1024);
            let mut ctx = RasterContext::new().unwrap();
            for at in [-10.0, 0.0, 25.0] {
                let g = group(2.0, at);
                let actual = cache.fill(&mut ctx, &g, &mut Default::default()).unwrap();
                let expected = rasterize_xy(&mut ctx, &g).unwrap();
                assert_eq!(actual.bounds(), expected.bounds());
                assert_eq!(actual.data(), expected.data());
            }
            assert_eq!(cache.stats().hits, 2);
            let g = group(2.0, 25.125);
            drop(cache.fill(&mut ctx, &g, &mut Default::default()).unwrap());
            assert_eq!(cache.stats().misses, 2);
            drop(
                cache
                    .fill(&mut ctx, &group(3.0, 25.125), &mut Default::default())
                    .unwrap(),
            );
            assert_eq!(cache.stats().misses, 3);
        }
        #[test]
        fn filter_key_covers_bytes_units_parameters_bounds_and_resource_checks() {
            let mut cache=MaskCache::new(64*1024);
            let limits=Limits::default();
            let f=|n|FiniteF64::new(n).unwrap();
            let spec=crate::effects::BlurSpec::new(f(2.0),f(1.5),[1.0,1.0],&limits).unwrap();
            let mask=|x:i32,xy:bool,first:u8| {
                let bounds=PixelRect::new(x,3,x+3,6).unwrap();
                let data:Arc<[u8]>=Arc::from([first,8,32,0,64,5,40,0,16,1,12,0]);
                Arc::new(if xy { Mask::new_xy(bounds,4,data) } else { Mask::new(bounds,4,data) }.unwrap())
            };
            for (x,xy,first) in [(0,true,1),(10,true,1),(10,false,1),(10,false,2)] {
                let input=mask(x,xy,first);
                let actual=cache.filter(&input,&spec,&limits).unwrap();
                let expected=crate::effects::apply_effects(&input,&spec,&limits).unwrap();
                assert_eq!(actual.bounds(),expected.bounds());
                assert_eq!(actual.stride(),expected.stride());
                assert_eq!(actual.is_xy_coverage(),expected.is_xy_coverage());
                assert_eq!(actual.data(),expected.data());
            }
            assert_eq!(cache.stats().hits,1);
            assert_eq!(cache.stats().misses,3);
            let input=mask(0,true,1);
            let other=crate::effects::BlurSpec::new(f(3.0),f(1.5),[1.0,1.0],&limits).unwrap();
            drop(cache.filter(&input,&other,&limits).unwrap());
            assert_eq!(cache.stats().misses,4);
            let tiny=limits.with_transient_bytes(1).unwrap();
            let a=crate::effects::apply_effects(&input,&spec,&tiny).unwrap_err();
            let b=cache.filter(&input,&spec,&tiny).unwrap_err();
            assert_eq!(a.code(),b.code());
            assert_eq!(a.resource_failure(),b.resource_failure());
        }
    }
}
#[cfg(feature = "native")]
pub(crate) use masks::MaskCache;
