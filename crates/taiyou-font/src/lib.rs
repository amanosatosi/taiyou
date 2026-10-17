//! Owned font data and private native typography adapters.
//!
//! The explicit native feature uses the T002 pinned source recipe, never system
//! discovery. Native/unsafe and shared-interface review remain acceptance gates.
#[cfg(feature="native")]
mod catalog;
#[cfg(feature="native")]
mod native;
#[cfg(feature="native")]
pub use catalog::{FontCatalogBuilder,fold_family,resolve_family};
#[cfg(feature="native")]
pub use native::FontContext;
