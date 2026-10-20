//! Immutable subtitle rendering stages.
//!
//! The explicit native feature connects evaluation, shaping, local layout,
//! owned geometry, grayscale coverage, FreeType border/shadow masks and scalar
//! composition through the single-worker CoreV1 coordinator.

pub mod collision;
pub mod collision_footprint;
mod compose;
pub mod effects;
mod evaluate;
pub mod geometry;
pub mod layout;
mod paint;
pub mod raster;
pub mod renderer;
pub mod stroke;
mod xy_wrap;
pub use compose::compose;
pub use evaluate::{evaluate, evaluate_active, evaluate_with_policy};
pub use layout::layout;
pub use paint::{blend_pixel, mul255};
#[cfg(feature = "native")]
pub mod shaping {
    pub use taiyou_font::{
        FontCatalogBuilder, FontContext, NativeWorkExperiment, ShapeOptions, shape_event,
        shape_event_advances_experiment,
    };
}
