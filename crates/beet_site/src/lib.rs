#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

pub mod codegen;
pub mod components;
pub mod layouts;

// doesnt have to be wasm, but at least not run when compiling server
// because stale references
#[path = "codegen/client_islands.rs"]
#[cfg(target_arch = "wasm32")]
pub mod client_islands;

pub use codegen::actions;

pub mod prelude {
	#[cfg(target_arch = "wasm32")]
	pub use super::client_islands::*;
	pub use super::codegen::actions;
	pub use super::codegen::actions::ActionsPlugin;
	pub use super::codegen::docs::DocsPlugin;
	pub use super::codegen::pages::*;
	pub use super::layouts::*;
	// pub use super::types::*;
	pub use super::*;
	pub use crate::codegen::route_path_tree;
	pub use crate::codegen::routes;
	pub use crate::components::*;
}
