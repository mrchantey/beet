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
			.add(ik_plugin);


		#[cfg(feature = "animation")]
		(builder = builder.add(crate::prelude::AnimationPlugin::default()));

		builder
	}
}
