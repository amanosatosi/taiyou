//! Owned font data and private native typography adapters.
//!
//! Native shaping uses the T002 pinned source recipe. SystemFontSource is an
//! explicit host admission step; immutable catalogs never discover fonts.
//! Native/unsafe and shared-interface review remain acceptance gates.
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
#[cfg(all(feature = "native", windows))]
pub use native::windows::WindowsFontSource;
#[cfg(all(feature = "native", windows))]
pub use native::windows::WindowsFontSource as SystemFontSource;
#[cfg(all(feature = "native", target_os = "linux"))]
pub use native::fontconfig::SystemFontSource;
#[cfg(all(feature = "native", target_os = "macos"))]
pub use native::coretext::SystemFontSource;
#[cfg(feature = "native")]
pub use native::{FontContext, NativeWorkExperiment, RasterContext};
