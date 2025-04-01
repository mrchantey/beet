#![feature(more_qualified_paths)]

pub mod components;
#[path = "codegen/route_tree.rs"]
pub mod route_tree;
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/routes.rs"]
pub mod routes;
#[cfg(target_arch = "wasm32")]
#[path = "codegen/wasm.rs"]
pub mod wasm;

pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
	pub use crate::route_tree::paths;
}
