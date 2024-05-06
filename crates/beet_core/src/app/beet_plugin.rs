use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

#[derive(Default)]
pub struct DefaultBeetPlugins {
	pub lifecycle: LifecyclePlugin,
	pub movement: MovementPlugin,
	pub steering: SteerPlugin,
	pub core: SomeFunPlugin,
}

impl DefaultBeetPlugins {
	pub fn new() -> Self {
		Self {
			lifecycle: Default::default(),
			movement: Default::default(),
			steering: Default::default(),
			core: Default::default(),
		}
	}
}

impl PluginGroup for DefaultBeetPlugins {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.lifecycle)
			.add(self.movement)
			.add(self.steering)
			.add(self.core)
	}
}
