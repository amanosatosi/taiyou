//! Immutable subtitle rendering stages.
//!
//! T013 provides a scalar compositor for explicit synthetic masks. Subtitle
//! rendering, fonts, geometry and rasterization remain unimplemented.

mod compose;
mod paint;
pub use compose::compose;
pub use paint::{blend_pixel, mul255};
