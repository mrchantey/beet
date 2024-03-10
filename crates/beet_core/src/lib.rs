pub use beet_ecs::*;
pub use beet_net::*;

pub mod api;
pub mod app;
pub mod core_nodes;
pub mod steering;

// currently required for action_list! macro to work
extern crate self as beet;

pub mod prelude {
	pub use crate::api::*;
	pub use crate::app::*;
	pub use crate::core_nodes::*;
	pub use crate::steering::actions::*;
	pub use crate::steering::algo::*;
	pub use crate::steering::*;
}
