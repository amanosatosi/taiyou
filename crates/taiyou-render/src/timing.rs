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
