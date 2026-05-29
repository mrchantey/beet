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
//! **Deferred — `BucketList`.** Blocked by (a) missing `Bucket`/`local_bucket`
//! infra in `beet_net`, (b) reactive `{Getter<T>}` block-child lift in
//! scene-`rsx!` (see `agent/plans/beet_design.md` § Outstanding scene-API
//! polish §4). Once both land, port `BucketList` as a synchronous `#[scene]`
//! constructor whose behavior (list/insert/remove) is async via `effect` +
//! `onclick` (see `tests/scene.rs::Counter` for the simpler pattern).

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
pub use table::*;
