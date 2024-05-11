#![feature(let_chains, anonymous_lifetime_in_impl_trait)]
#[cfg(feature = "animation")]
pub mod animation;
pub mod app;
pub mod movement;
pub mod robotics;
pub mod steer;

pub mod prelude {

	// pub extern crate beet_ecs as beet;
	#[cfg(feature = "animation")]
	pub use crate::animation::*;
	pub use crate::app::*;
	pub use crate::movement::*;
	pub use crate::robotics::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::*;
}
