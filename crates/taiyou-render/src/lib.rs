//! Immutable subtitle rendering stages.
//!
//! The explicit native feature connects evaluation, shaping, local layout,
//! owned geometry, grayscale fill rasterization and scalar composition through
//! the T014 single-worker CoreV1 coordinator. Effects remain unsupported.

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
