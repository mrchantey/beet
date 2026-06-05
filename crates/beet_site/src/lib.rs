#![doc = include_str!("../README.md")]

// ── codegen generator ────────────────────────────────────────────────────────
// Compiles without the generated `src/codegen` modules so it can bootstrap them.
#[cfg(feature = "codegen")]
mod launch;
#[cfg(feature = "codegen")]
pub use launch::run_codegen;

// ── render targets (web + terminal) ──────────────────────────────────────────
#[cfg(feature = "render")]
pub mod layouts;
#[cfg(feature = "render")]
mod server;

// generated route modules (see `run_codegen`)
#[cfg(feature = "render")]
#[path = "codegen/blog/mod.rs"]
mod blog_codegen;
#[cfg(feature = "render")]
#[path = "codegen/docs/mod.rs"]
mod docs_codegen;
#[cfg(feature = "render")]
#[path = "codegen/pages.rs"]
mod pages_codegen;
#[cfg(feature = "render")]
#[path = "codegen/route_tree.rs"]
mod route_tree;

pub mod prelude {
	#[cfg(feature = "render")]
	pub use crate::blog_codegen::*;
	#[cfg(feature = "render")]
	pub use crate::docs_codegen::*;
	#[cfg(feature = "render")]
	pub use crate::layouts::*;
	#[cfg(feature = "render")]
	pub use crate::pages_codegen::*;
	#[cfg(feature = "render")]
	pub use crate::route_tree::routes;
	#[cfg(feature = "render")]
	pub use crate::server::*;
}
