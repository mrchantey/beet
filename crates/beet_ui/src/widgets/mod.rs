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
//! **Reactive substrate.** State lives in documents:
//! [`TypedFieldRef`](crate::prelude::TypedFieldRef) for a single typed atom and
//! [`ReactiveChildren`](crate::prelude::ReactiveChildren) for a list field that
//! materializes one child per item. The full loop — native event then document
//! mutation then change-detected rebuild — is proven by `native_event_drives_list`
//! in `document/reactive_children.rs`, with zero render-target coupling.
//!
//! A DOM widget or a `BlobStore`-backed list is a render-target / storage
//! binding layered on top, not a gap in the substrate: a render backend triggers
//! the native events (see `input/pointer.rs`), and an async store can sync
//! `BlobStore::list()` into a `Vec<_>` field via
//! [`AsyncWorld`](beet_core::prelude::AsyncWorld) when that integration is wanted.

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
