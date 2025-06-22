#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

pub mod codegen;
pub mod components;
mod layouts;
pub use layouts::*;
// #[path = "codegen/route_tree.rs"]
// pub mod route_tree;
// #[cfg(not(target_arch = "wasm32"))]
// #[path = "codegen/server_actions.rs"]
// pub mod server_actions;
// #[cfg(target_arch = "wasm32")]
// #[path = "codegen/wasm.rs"]
// pub mod wasm;
pub mod prelude {
	pub use super::codegen::docs::*;
	pub use super::codegen::pages::*;
	pub use super::*;
	pub use crate::codegen::routes;
	pub use crate::codegen::route_path_tree;
	pub use crate::components::*;
}
