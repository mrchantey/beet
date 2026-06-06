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
pub mod layouts;
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod server;
#[cfg(all(feature = "render", not(feature = "codegen")))]
mod style;

// generated route modules (see `run_codegen`)
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/blog/mod.rs"]
mod blog_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/docs/mod.rs"]
mod docs_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/pages.rs"]
mod pages_codegen;
#[cfg(all(feature = "render", not(feature = "codegen")))]
#[path = "codegen/route_tree.rs"]
mod route_tree;

pub mod prelude {
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::blog_codegen::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::docs_codegen::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::layouts::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::pages_codegen::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::route_tree::routes;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::server::*;
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use crate::style::*;
	// the library prelude, so a page needs only `use crate::prelude::*`.
	#[cfg(all(feature = "render", not(feature = "codegen")))]
	pub use beet::prelude::*;
}
