//! Opt-in observational timings, owned by one renderer. Never used for decisions.
use std::time::Duration;
#[cfg(feature = "native")]
use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct StageTimings {
    pub enabled: bool,
    pub total: Duration,
    pub evaluation: Duration,
    pub shaping: Duration,
    pub layout: Duration,
    pub collision: Duration,
    pub geometry: Duration,
    pub clip: Duration,
    /// Raster, stroke and effects, including coverage grouping.
    pub masks: Duration,
    /// Subset of masks: Gaussian/edge filtering only.
    pub effects: Duration,
    pub base_outlines: Duration,
    pub scan_conversion: Duration,
    pub border_expansion: Duration,
    pub span_union: Duration,
    /// Subsets of scan/union work. Allocation overlaps its owning stage.
    pub edge_generation: Duration,
    pub edge_sorting: Duration,
    pub span_generation: Duration,
    pub row_bucketing: Duration,
    pub span_sorting: Duration,
    pub interval_merging: Duration,
    pub raster_allocation: Duration,
    pub bitmap_clear: Duration,
    /// Coverage arithmetic and its destination writes are one fused loop.
    pub coverage_writes: Duration,
    pub bitmap_publication: Duration,
    pub raster_allocations: u64,
    pub raster_reallocations: u64,
    pub raster_capacity_growth_bytes: u64,
    pub coverage_write_samples: u64,
    pub tiled_fill: Duration,
    pub tiled_fills: u64,
    pub tiled_borders: u64,
    pub tile_simple_borders: u64,
    pub tile_fallbacks: u64,
    pub tile_empty_regions: u64,
    pub tile_solid_regions: u64,
    pub tile_halfplanes: u64,
    pub tile_complex: u64,
    pub tile_scratch_peak_bytes: u64,
    pub mask_generation: Duration,
    pub mask_combination: Duration,
    pub span_rows: u64,
    pub expanded_candidates: u64,
    pub expanded_candidates_without_reuse: u64,
    pub paint: Duration,
    pub composition: Duration,
    // Deterministic producer resource witness, not timing-driven policy.
    #[cfg(feature = "native")]
    pub(crate) raster_peak_bytes: u64,
    #[cfg(feature = "native")]
    pub(crate) raster_required_work: u64,
}
#[cfg(feature = "native")]
impl StageTimings {
    pub(crate) fn start(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }
    pub(crate) fn elapsed(start: Option<Instant>) -> Duration {
        start.map_or(Duration::ZERO, |t| t.elapsed())
    }
    pub(crate) fn raster_peak(&mut self, bytes: u64, work: u64) {
        self.raster_peak_bytes = self.raster_peak_bytes.max(bytes);
        self.raster_required_work = self.raster_required_work.max(work);
    }
    pub(crate) fn merge_raster(&mut self, other: &Self) {
        self.scan_conversion += other.scan_conversion;
        self.border_expansion += other.border_expansion;
        self.span_union += other.span_union;
        self.edge_generation += other.edge_generation;
        self.edge_sorting += other.edge_sorting;
        self.span_generation += other.span_generation;
        self.row_bucketing += other.row_bucketing;
        self.span_sorting += other.span_sorting;
        self.interval_merging += other.interval_merging;
        self.raster_allocation += other.raster_allocation;
        self.bitmap_clear += other.bitmap_clear;
        self.coverage_writes += other.coverage_writes;
        self.bitmap_publication += other.bitmap_publication;
        self.raster_allocations = self
            .raster_allocations
            .saturating_add(other.raster_allocations);
        self.raster_reallocations = self
            .raster_reallocations
            .saturating_add(other.raster_reallocations);
        self.raster_capacity_growth_bytes = self
            .raster_capacity_growth_bytes
            .saturating_add(other.raster_capacity_growth_bytes);
        self.coverage_write_samples = self
            .coverage_write_samples
            .saturating_add(other.coverage_write_samples);
        self.tiled_fill += other.tiled_fill;
        self.tiled_fills = self.tiled_fills.saturating_add(other.tiled_fills);
        self.tiled_borders = self.tiled_borders.saturating_add(other.tiled_borders);
        self.tile_simple_borders = self.tile_simple_borders.saturating_add(other.tile_simple_borders);
        self.tile_fallbacks = self.tile_fallbacks.saturating_add(other.tile_fallbacks);
        self.tile_empty_regions = self
            .tile_empty_regions
            .saturating_add(other.tile_empty_regions);
        self.tile_solid_regions = self
            .tile_solid_regions
            .saturating_add(other.tile_solid_regions);
        self.tile_halfplanes = self.tile_halfplanes.saturating_add(other.tile_halfplanes);
        self.tile_complex = self.tile_complex.saturating_add(other.tile_complex);
        self.tile_scratch_peak_bytes = self
            .tile_scratch_peak_bytes
            .max(other.tile_scratch_peak_bytes);
        self.mask_generation += other.mask_generation;
        self.span_rows = self.span_rows.saturating_add(other.span_rows);
        self.expanded_candidates = self
            .expanded_candidates
            .saturating_add(other.expanded_candidates);
        self.expanded_candidates_without_reuse = self
            .expanded_candidates_without_reuse
            .saturating_add(other.expanded_candidates_without_reuse);
        self.raster_peak(other.raster_peak_bytes, other.raster_required_work);
    }
}
