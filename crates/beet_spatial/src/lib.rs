#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(feature = "animation")]
pub mod animation;
#[cfg(feature = "assets")]
pub mod asset_actions;
pub mod extensions;
pub mod inverse_kinematics;
pub mod movement;
pub mod plugins;
pub mod procedural_animation;
pub mod robotics;
pub mod steer;
#[cfg(feature = "ui")]
pub mod ui;

pub mod prelude {
	#[cfg(feature = "animation")]
	pub use crate::animation::*;
	#[cfg(feature = "assets")]
	pub use crate::asset_actions::*;
	// todo wait for serializable asset handles
	// pub use crate::bevyhub::*;
	pub use crate::extensions::*;
	pub use crate::inverse_kinematics::*;
	pub use crate::movement::*;
	pub use crate::plugins::*;
	pub use crate::procedural_animation::*;
	pub use crate::robotics::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::*;
	#[cfg(feature = "ui")]
	pub use crate::ui::*;
}
