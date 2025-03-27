#![feature(more_qualified_paths)]
#![cfg(not(feature = "setup"))]




#[cfg(not(feature = "setup"))]
pub mod components;

#[cfg(not(feature = "setup"))]
pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
}

#[cfg(all(target_arch = "wasm32", not(feature = "setup")))]
#[path = "codegen/wasm.rs"]
pub mod wasm;

#[cfg(all(not(target_arch = "wasm32"), not(feature = "setup")))]
#[path = "codegen/routes.rs"]
pub mod routes;

#[cfg(not(feature = "setup"))]
#[path = "codegen/route_tree.rs"]
pub mod route_tree;
