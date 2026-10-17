//! Immutable subtitle rendering stages.
//!
//! T013 provides a scalar compositor for explicit synthetic masks. Subtitle
//! rendering, geometry and rasterization remain unimplemented. The explicit
//! native feature exposes the T009 font/shaping pipeline.

mod compose;
mod evaluate;
mod paint;
pub use compose::compose;
pub use evaluate::{evaluate, evaluate_active, evaluate_with_policy};
pub use paint::{blend_pixel, mul255};
#[cfg(feature = "native")]
pub mod shaping {
    pub use taiyou_font::{FontCatalogBuilder, FontContext, ShapeOptions, shape_event};
}
