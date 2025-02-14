#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]
// #[cfg(feature = "animation")]
// pub mod animation;
#[cfg(feature = "assets")]
pub mod asset_actions;
mod extensions;
pub mod inverse_kinematics;
pub mod movement;
pub mod procedural_animation;
pub mod robotics;
pub mod steer;
pub mod steer_actions;
#[cfg(feature = "ui")]
pub mod ui;

/// Re-exports of the most commonly used types and functions in `beet_spatial`.
pub mod prelude {
	pub use super::*;
	// #[cfg(feature = "animation")]
	// pub use crate::animation::*;
	#[cfg(feature = "assets")]
	pub use crate::asset_actions::*;
	// todo wait for construct
	// pub use crate::bevyhub::*;
	pub use crate::extensions::*;
	pub use crate::inverse_kinematics::*;
	pub use crate::movement::*;
	pub use crate::procedural_animation::*;
	pub use crate::robotics::*;
	pub use crate::steer::*;
	pub use crate::steer_actions::*;
	#[cfg(feature = "ui")]
	pub use crate::ui::*;
}
use crate::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

/// Plugins used for most beet apps.
#[derive(Default, Clone)]
pub struct BeetSpatialPlugins;

impl PluginGroup for BeetSpatialPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>()
		.add(movement_plugin)
		.add(procedural_animation_plugin)
		.add(steer_plugin)
		.add(ik_plugin)
		/*-*/;

		// #[cfg(feature = "animation")]
		// (builder = builder.add(crate::prelude::AnimationPlugin::default()));
		builder
	}
}

/// doctest reexports and utilities
#[cfg(feature = "_doctest")]
#[allow(ambiguous_glob_reexports)]
pub mod doctest {
	pub use crate::prelude::*;
	pub use beet_flow::prelude::*;
	pub use bevy::prelude::*;
	/// for docs, create a world with BeetFlowPlugin
	/// ```
	/// use beet_spatial::doctest::*;
	/// let world = world();
	/// ```
	#[cfg(feature = "_doctest")]
	pub fn world() -> World {
		let mut app = App::new();
		app.add_plugins((BeetFlowPlugin::default(), BeetSpatialPlugins));
		let world = std::mem::take(app.world_mut());
		world
	}
}
