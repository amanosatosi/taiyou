//! Owned font data and private native typography adapters.
//!
//! The explicit native feature uses the T002 pinned source recipe, never system
//! discovery. Native/unsafe and shared-interface review remain acceptance gates.
#[cfg(feature = "native")]
mod catalog;
#[cfg(feature = "native")]
mod native;
#[cfg(feature = "native")]
mod shape;
#[cfg(feature = "native")]
pub use shape::{
    ResolvedRun, ShapeOptions, resolve_runs, shape, shape_event, shape_event_advances_experiment,
};
pub mod source_map;
#[cfg(feature = "native")]
pub use catalog::{
    FontCatalogBuilder, ValidatedFontCatalog, fold_family, resolve_family, validate_catalog,
};
#[cfg(feature = "native")]
pub use native::{FontContext, NativeWorkExperiment, RasterContext};
