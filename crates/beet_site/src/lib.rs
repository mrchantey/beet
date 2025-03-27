#![feature(more_qualified_paths)]

pub mod components;

pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
}

#[cfg(target_arch = "wasm32")]
#[path = "codegen/wasm.rs"]
pub mod wasm;

#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/routes.rs"]
pub mod routes;
