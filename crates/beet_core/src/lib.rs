#![feature(let_chains, anonymous_lifetime_in_impl_trait)]
pub mod app;
pub mod core_module;
pub mod movement;
pub mod robotics;
pub mod steering;

pub extern crate beet_ecs as beet;
pub mod prelude {

	// pub extern crate beet_ecs as beet;
	pub use crate::app::*;
	pub use crate::core_module::*;
	pub use crate::movement::*;
	pub use crate::robotics::*;
	pub use crate::steering::algo::*;
	pub use crate::steering::steering_actions::*;
	pub use crate::steering::*;
}
