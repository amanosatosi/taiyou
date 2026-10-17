//! Checked units and immutable subtitle records for the CoreV1 development profile.
//!
//! Mutable ingestion/diagnostic builders are owned by one coordinator. Published
//! records contain no interior mutability, native handles, or renderer state.
//!
//! Checked numeric values cannot be forged through tuple fields:
//! ```compile_fail
//! let invalid = taiyou_types::FiniteF64(f64::NAN);
//! ```
//! Published snapshot fields cannot be replaced by consumers:
//! ```compile_fail
//! fn mutate(track: &mut taiyou_types::Track) {
//!     track.events = std::sync::Arc::from([]);
//! }
//! ```
//! Session identity owners cannot be cloned to recycle their counter:
//! ```compile_fail
//! let owner = taiyou_types::TrackIngestor::new();
//! let duplicate = owner.clone();
//! ```

mod error;
mod evaluation;
mod fonts;
mod scene;
mod track;
mod units;

pub use error::*;
pub use evaluation::*;
pub use fonts::*;
pub use scene::*;
pub use track::*;
pub use units::*;
