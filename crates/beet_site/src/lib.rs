#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

// #![feature(more_qualified_paths)]

#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/actions.rs"]
pub mod actions;
#[path = "codegen/client_actions.rs"]
pub mod client_actions;
pub mod codegen;
pub mod components;
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/docs.rs"]
pub mod docs;
#[cfg(not(target_arch = "wasm32"))]
#[path = "codegen/pages.rs"]
pub mod pages;
// #[path = "codegen/route_tree.rs"]
// pub mod route_tree;
// #[cfg(not(target_arch = "wasm32"))]
// #[path = "codegen/server_actions.rs"]
// pub mod server_actions;
// #[cfg(target_arch = "wasm32")]
// #[path = "codegen/wasm.rs"]
// pub mod wasm;

pub mod prelude {
	pub use super::pages::*;
	pub use super::*;
	pub use crate::client_actions::root as actions;
	pub use crate::codegen::root as paths;
	pub use crate::codegen::route_path_tree;
	pub use crate::components::*;
}
