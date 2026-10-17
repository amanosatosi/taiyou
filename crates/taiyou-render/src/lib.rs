//! Immutable subtitle rendering stages.
//!
//! T013 provides a scalar compositor for explicit synthetic masks. Subtitle
//! rendering, fonts, geometry and rasterization remain unimplemented.

mod paint;
mod compose;
pub use paint::{mul255, blend_pixel};
pub use compose::compose;
