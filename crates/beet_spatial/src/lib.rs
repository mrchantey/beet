#![feature(let_chains, anonymous_lifetime_in_impl_trait)]
#[cfg(feature = "render")]
pub mod animation;
pub mod app;
#[cfg(feature = "render")]
pub mod asset_actions;
pub mod movement;
pub mod robotics;
pub mod scenes;
pub mod steer;
#[cfg(feature = "render")]
pub mod ui;

pub mod prelude {
	#[cfg(feature = "render")]
	pub use crate::animation::*;
	pub use crate::app::*;
	#[cfg(feature = "render")]
	pub use crate::asset_actions::*;
	pub use crate::movement::*;
	pub use crate::robotics::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::*;
	#[cfg(feature = "render")]
	pub use crate::ui::*;
}
