#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]

mod beet_router_plugin;
mod input;
mod interface_navigation;
mod interface_plugin;
mod media;
mod navigate;
mod route_tree;
mod router_plugin;
mod scene_route;
mod scene_route_query;
mod server;
mod tools;


/// Exports the most commonly used items.
pub mod prelude {
	pub use crate::beet_router_plugin::*;
	pub use crate::input::*;
	pub use crate::interface_navigation::*;
	pub use crate::interface_plugin::*;
	pub use crate::media::*;
	pub use crate::navigate::*;
	pub use crate::route_tree::*;
	pub use crate::router_plugin::*;
	pub use crate::scene_route::*;
	pub use crate::scene_route_query::*;
	pub use crate::server::*;
	pub use crate::tools::*;
}
