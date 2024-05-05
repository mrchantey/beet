use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy::time::TimePlugin;

/// The plugin required for most beet apps
pub struct BeetMinimalPlugin;
impl Plugin for BeetMinimalPlugin {
	fn build(&self, app: &mut App) { app.add_plugins(TimePlugin); }
}
#[derive(Default)]
pub struct DefaultBeetPlugins {
	pub lifecycle: LifecyclePlugin,
	pub steering: SteerPlugin,
	pub movement: MovementPlugin,
	pub core: SomeFunPlugin,
}

impl DefaultBeetPlugins {
	pub fn new() -> Self {
		Self {
			lifecycle: Default::default(),
			steering: Default::default(),
			movement: Default::default(),
			core: Default::default(),
		}
	}
}

impl PluginGroup for DefaultBeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.lifecycle)
			.add(self.steering)
			.add(self.movement)
			.add(self.core)
	}
}
