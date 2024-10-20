use crate::prelude::*;
use beet_flow::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;



/// Includes the following
/// - [BeetSpatialPlugins]
/// - [BeetMinimalPlugins]
#[derive(Debug, Clone, Default)]
pub struct BeetDefaultPlugins;

impl PluginGroup for BeetDefaultPlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add_group(BeetSpatialPlugins::default())
			.add_group(BeetMinimalPlugins)
	}
}

/// Includes the following
/// - [LifecyclePlugin]
#[derive(Debug, Clone, Default)]
pub struct BeetMinimalPlugins;

impl PluginGroup for BeetMinimalPlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>().add(LifecyclePlugin::default())
	}
}
