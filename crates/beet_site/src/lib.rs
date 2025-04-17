#![feature(more_qualified_paths)]

pub mod components;
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/docs.rs"]
pub mod docs;
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/pages.rs"]
pub mod pages;
#[path = "codegen/route_tree.rs"]
pub mod route_tree;
#[cfg(target_arch = "wasm32")]
#[path = "codegen/wasm.rs"]
pub mod wasm;
// #[cfg(target_arch = "wasm32")]
// #[path = "codegen/actions.rs"]
// pub mod actions;

pub mod prelude {
	pub use super::*;
	pub use crate::components::*;
	pub use crate::route_tree::root as paths;
}
