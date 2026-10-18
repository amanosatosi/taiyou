//! Immutable subtitle rendering stages.
//!
//! T013 provides a scalar compositor for explicit synthetic masks. Subtitle
//! rendering and rasterization remain unimplemented. T011 provides owned geometry. The explicit
//! native feature exposes the T009 font/shaping pipeline.

mod compose;
mod evaluate;
pub mod geometry;
pub mod layout;
mod paint;
pub mod raster;
pub mod renderer;
pub use compose::compose;
pub use evaluate::{evaluate, evaluate_active, evaluate_with_policy};
pub use layout::layout;
pub use paint::{blend_pixel, mul255};
#[cfg(feature = "native")]
pub mod shaping {
    pub use taiyou_font::{FontCatalogBuilder, FontContext, ShapeOptions, shape_event};
}
