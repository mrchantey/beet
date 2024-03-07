pub use beet_ecs::*;
pub use beet_net::*;

pub mod app;
pub mod api;
pub mod core_nodes;

// currently required for action_list! macro to work
extern crate self as beet;

pub mod prelude {
	pub use crate::app::*;
	pub use crate::api::*;
	pub use crate::core_nodes::*;
}
