#![feature(let_chains, anonymous_lifetime_in_impl_trait)]
#[cfg(feature = "animation")]
pub mod animation;
#[cfg(feature = "assets")]
pub mod asset_actions;
pub mod procedural_animation;
pub mod inverse_kinematics;
pub mod movement;
pub mod plugins;
pub mod extensions;
pub mod robotics;
pub mod steer;
#[cfg(feature = "ui")]
pub mod ui;

pub mod prelude {
	#[cfg(feature = "animation")]
	pub use crate::animation::*;
	#[cfg(feature = "assets")]
	pub use crate::asset_actions::*;
	pub use crate::procedural_animation::*;
	pub use crate::inverse_kinematics::*;
	pub use crate::movement::*;
	pub use crate::plugins::*;
	pub use crate::extensions::*;
	pub use crate::robotics::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::*;
	#[cfg(feature = "ui")]
	pub use crate::ui::*;
}
