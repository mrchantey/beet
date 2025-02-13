#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#[cfg(feature = "animation")]
pub mod animation;
#[cfg(feature = "assets")]
pub mod asset_actions;
pub mod extensions;
pub mod inverse_kinematics;
pub mod movement;
pub mod procedural_animation;
pub mod robotics;
pub mod steer;
#[cfg(feature = "ui")]
pub mod ui;

pub mod prelude {
	pub use super::*;
	#[cfg(feature = "animation")]
	pub use crate::animation::*;
	#[cfg(feature = "assets")]
	pub use crate::asset_actions::*;
	// todo wait for construct
	// pub use crate::bevyhub::*;
	pub use crate::extensions::*;
	pub use crate::inverse_kinematics::*;
	pub use crate::movement::*;
	pub use crate::procedural_animation::*;
	pub use crate::robotics::*;
	pub use crate::steer::algo::*;
	pub use crate::steer::steer_actions::*;
	pub use crate::steer::*;
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
		.add(MovementPlugin::default())
		.add(SteerPlugin::default())
		.add(ik_plugin)
		/*-*/;

		#[cfg(feature = "render")]
		builder.add(spatial_observers_plugin);


		#[cfg(feature = "animation")]
		(builder = builder.add(crate::prelude::AnimationPlugin::default()));

		builder
	}
}

#[cfg(feature = "render")]
pub fn spatial_observers_plugin(app: &mut App) {
	app.add_plugins(beet_flow::prelude::ActionPlugin::<(
		InsertOnRun<Visibility>,
		InsertOnRunResult<Visibility>,
	)>::default());
}
