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
