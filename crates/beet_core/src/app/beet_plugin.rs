use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

/// Plugins used for most beet apps.
#[derive(Default)]
pub struct DefaultBeetPlugins {
	pub lifecycle: LifecyclePlugin,
	pub movement: MovementPlugin,
	pub steering: SteerPlugin,
}

impl DefaultBeetPlugins {
	pub fn new() -> Self {
		Self {
			lifecycle: Default::default(),
			movement: Default::default(),
			steering: Default::default(),
		}
	}
}

impl PluginGroup for DefaultBeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.lifecycle)
			.add(self.movement)
			.add(self.steering)
	}
}
