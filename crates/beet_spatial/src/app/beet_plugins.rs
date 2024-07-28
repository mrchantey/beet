use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

/// Plugins used for most beet apps.
#[derive(Default)]
pub struct BeetPlugins;

impl PluginGroup for BeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		#[allow(unused_mut)]
		let mut builder = PluginGroupBuilder::start::<Self>()
			.add(LifecyclePlugin::default())
			.add(MovementPlugin::default())
			.add(SteerPlugin::default());


		#[cfg(feature = "render")]
		(builder = builder.add(crate::prelude::AnimationPlugin::default()));

		builder
	}
}
