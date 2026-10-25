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
    pub paint: Duration,
    pub composition: Duration,
}
#[cfg(feature = "native")]
impl StageTimings {
    pub(crate) fn start(&self) -> Option<Instant> {
        self.enabled.then(Instant::now)
    }
    pub(crate) fn elapsed(start: Option<Instant>) -> Duration {
        start.map_or(Duration::ZERO, |t| t.elapsed())
    }
}
