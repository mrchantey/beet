//! Reusable `#[scene]` function-component widgets.
//!
//! Widgets emit *semantic* classes via [`Classes`](crate::token::Classes)
//! (never `class="…"` strings); the active rule set (Material Design 3 via
//! `MaterialStylePlugin` today) maps those classes to design tokens. See
//! `agent/plans/beet_design.md` for the authoring model.
//!
//! Gated behind the `scene` feature; rendering targets and styling come from
//! the same DOM + rule machinery as parsed HTML.
//!
//! **Pending — `BlobStoreList`.** The reactive substrate it needs now exists:
//! [`DocState`](crate::prelude::DocState) for the live path list and
//! [`ReactiveChildren`](crate::prelude::ReactiveChildren) for the per-blob rows.
//! What remains is the async glue: `onclick` handlers (see
//! `tests/scene.rs::Counter`) that run `BlobStore` ops via
//! [`AsyncWorld`](beet_core::prelude::AsyncWorld) and write the refreshed
//! `BlobStore::list()` back into the `DocState`, re-spawning the rows.

#[cfg(feature = "net")]
mod analytics;
mod button;
mod color_scheme;
mod error_text;
mod footer;
mod form_controls;
mod head;
mod header;
mod layout;
mod preflight;
mod sidebar;
#[cfg(feature = "style")]
mod stylesheet;
mod table;

#[cfg(feature = "net")]
pub use analytics::*;
pub use button::*;
pub use color_scheme::*;
pub use error_text::*;
pub use footer::*;
pub use form_controls::*;
pub use head::*;
pub use header::*;
pub use layout::*;
pub use preflight::*;
pub use sidebar::*;
#[cfg(feature = "style")]
pub use stylesheet::*;
pub use table::*;
