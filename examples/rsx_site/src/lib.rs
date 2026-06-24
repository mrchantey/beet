#![doc = include_str!("../README.md")]

// ── codegen generator ────────────────────────────────────────────────────────
// Compiles without the generated `src/codegen` modules so it can bootstrap them.
#[cfg(feature = "codegen")]
mod launch;
#[cfg(feature = "codegen")]
pub use launch::run_codegen;

// ── render targets (web + terminal) ──────────────────────────────────────────
// Gated `not(codegen)` as well as `render`: the `codegen` feature layers onto
// the default `render` features, so excluding the render modules (and the
// generated includes below) lets `cargo run --features codegen` bootstrap the
// `src/codegen` modules even when they do not yet exist.
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod layout;
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod server;
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod shared;
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod style;

// generated route modules (see `run_codegen`)
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/pages.rs"]
mod pages_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/content.rs"]
mod content_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/actions.rs"]
mod actions_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/client_actions.rs"]
pub mod client_actions;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/route_tree.rs"]
mod route_tree;

pub mod prelude {
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::actions_codegen::*;
	// the generated client-action callers, kept as a named module so the client
	// `client_actions::add` does not collide with the server `add` handler.
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::client_actions;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::content_codegen::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::layout::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::pages_codegen::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::route_tree::routes;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::server::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::shared::*;
	// the site-local `classes` module is intentionally *not* re-exported: a bare
	// `classes::` resolves to the library set, and a site-local class is reached
	// by its full path (`crate::style::classes::DESIGN_ROW`).
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::style::design_row_rule;
}
