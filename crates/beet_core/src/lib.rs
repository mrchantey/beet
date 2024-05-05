#![feature(let_chains, anonymous_lifetime_in_impl_trait)]
pub mod app;
pub mod some_fun_plugin;
pub mod movement;
pub mod robotics;
pub mod steer;

pub extern crate beet_ecs as beet;
pub mod prelude {

	// pub extern crate beet_ecs as beet;
	pub use crate::app::*;
	pub use crate::some_fun_plugin::*;
	pub use crate::movement::*;
	pub use crate::robotics::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::*;
}
