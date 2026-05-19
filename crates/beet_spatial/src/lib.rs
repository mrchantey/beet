#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

beet_core::test_main!();

#[cfg(feature = "bevy_default")]
pub mod animation;
#[cfg(feature = "bevy_default")]
pub mod asset_actions;
mod extensions;
pub mod inverse_kinematics;
pub mod movement;
pub mod procedural_animation;
pub mod robotics;
pub mod steer;
pub mod steer_actions;
#[cfg(feature = "bevy_default")]
pub mod ui;

/// Re-exports of the most commonly used types and functions in `beet_spatial`.
pub mod prelude {
	pub use super::BeetSpatialPlugins;
	pub use super::PostTickSet;
	pub use super::PreTickSet;
	pub use super::TickSet;
	#[cfg(feature = "bevy_default")]
	pub use crate::animation::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::asset_actions::*;
	pub use crate::extensions::*;
	pub use crate::inverse_kinematics::*;
	pub use crate::movement::*;
	pub use crate::procedural_animation::*;
	pub use crate::robotics::*;
	pub use crate::steer::*;
	pub use crate::steer_actions::*;
	#[cfg(feature = "bevy_default")]
	pub use crate::ui::*;
}
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::app::PluginGroupBuilder;

/// Runs before [`TickSet`], for systems that prepare state for the tick
/// (e.g. spawning or repositioning agents).
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;

/// Per-frame steering and movement behavior systems run in this set,
/// applying [`Impulse`] and [`Force`] contributions before integration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;

/// Force integration runs in this set, after all [`TickSet`]
/// contributions have been applied.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;

/// Plugins used for most beet spatial apps.
#[derive(Default, Clone)]
pub struct BeetSpatialPlugins;

impl PluginGroup for BeetSpatialPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>()
			.add(spatial_set_plugin)
			.add(movement_plugin)
			.add(procedural_animation_plugin)
			.add(steer_plugin)
			.add(ik_plugin)
		/*-*/;

		#[cfg(feature = "bevy_default")]
		(builder = builder.add(crate::prelude::AnimationFlowPlugin));
		builder
	}
}

/// Orders [`PreTickSet`] → [`TickSet`] → [`PostTickSet`] in [`Update`].
fn spatial_set_plugin(app: &mut App) {
	app.configure_sets(Update, TickSet.after(PreTickSet))
		.configure_sets(Update, PostTickSet.after(TickSet));
}
