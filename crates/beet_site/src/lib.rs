#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]

#[cfg(any(feature = "server", feature = "launch"))]
pub mod infra;

#[cfg(any(feature = "server", feature = "client"))]
#[path = "codegen/client_actions.rs"]
pub mod client_actions;

#[cfg(any(feature = "server", feature = "client"))]
#[path = "codegen/route_tree.rs"]
pub mod route_tree;

#[cfg(any(feature = "server", feature = "client"))]
// #[cfg(feature = "server")]
pub mod codegen;

#[cfg(any(feature = "server", feature = "client"))]
pub mod components;

#[cfg(any(feature = "server", feature = "client"))]
pub mod layouts;

#[cfg(feature = "launch")]
mod launch;
#[cfg(feature = "server")]
mod server;

#[cfg(any(feature = "server", feature = "client"))]
pub use crate::client_actions::routes as actions;

pub mod prelude {
	#[cfg(any(feature = "server", feature = "client"))]
	pub use super::actions;
	#[cfg(feature = "server")]
	pub use crate::codegen::actions::actions_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::blog::blog_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::docs::docs_routes;
	#[cfg(feature = "server")]
	pub use crate::codegen::pages::pages_routes;
	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::components::*;
	// #[cfg(any(feature = "server", feature = "launch"))]
	// pub use crate::infra::*;
	#[cfg(feature = "launch")]
	pub use crate::launch::*;
	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::layouts::*;
	#[cfg(any(feature = "server", feature = "client"))]
	pub use crate::route_tree::routes;
	#[cfg(feature = "server")]
	pub use crate::server::*;
}
