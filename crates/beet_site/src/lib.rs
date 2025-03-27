#![feature(more_qualified_paths)]

pub mod components;
#[cfg(not(target_arch = "wasm32"))]
#[path = "./codegen/routes.rs"]
pub mod routes;

#[cfg(target_arch = "wasm32")]
#[path = "./codegen/routes_wasm.rs"]
pub mod routes;

pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
}
