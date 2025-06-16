// #![feature(more_qualified_paths)]

// #[path = "codegen/client_actions.rs"]
// pub mod client_actions;
// pub mod components;
// #[cfg(not(target_arch = "wasm32"))]
// #[path = "codegen/docs.rs"]
// pub mod docs;
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
	pub use super::*;
	// 	pub use crate::client_actions::root as actions;
	// 	pub use crate::components::*;
	// 	pub use crate::route_tree::root as paths;
}
