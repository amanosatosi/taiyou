//! Immutable subtitle rendering stages.
//!
//! T013 provides a scalar compositor for explicit synthetic masks. Subtitle
//! rendering, fonts, geometry and rasterization remain unimplemented.

mod compose;
mod evaluate;
mod paint;
pub use compose::compose;
pub use evaluate::{evaluate, evaluate_active, evaluate_with_policy};
pub use paint::{blend_pixel, mul255};
