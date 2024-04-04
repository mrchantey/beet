use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy::time::TimePlugin;
use std::marker::PhantomData;

/// The plugin required for most beet apps
pub struct BeetMinimalPlugin;
impl Plugin for BeetMinimalPlugin {
	fn build(&self, app: &mut App) { app.add_plugins(TimePlugin); }
}

pub struct DefaultBeetPlugins<T: ActionList> {
	pub types: BeetTypesPlugin<T>,
	pub systems: BeetSystemsPlugin<T, Update>,
	pub steering: SteeringPlugin,
	pub message: BeetMessagePlugin<T>,
	pub core: CorePlugin,
}

impl<T: ActionList> DefaultBeetPlugins<T> {
	pub fn new() -> Self {
		Self {
			types: BeetTypesPlugin(default()),
			systems: BeetSystemsPlugin::default(),
			steering: SteeringPlugin::default(),
			message: BeetMessagePlugin(default()),
			core: CorePlugin::default(),
		}
	}
}

impl<T: ActionList> PluginGroup for DefaultBeetPlugins<T> {
	fn build(self) -> PluginGroupBuilder {
		PluginGroupBuilder::start::<Self>()
			.add(self.types)
			.add(self.systems)
			.add(self.steering)
			.add(self.message)
			.add(self.core)
	}
}


#[derive(Default)]
pub struct BeetTypesPlugin<T: ActionList>(pub PhantomData<T>);

impl<T: ActionList> Plugin for BeetTypesPlugin<T> {
	fn build(&self, app: &mut App) {
		T::register_components(app.world_mut());
		T::register_types(
			&mut app.world().resource::<AppTypeRegistry>().write(),
		);
	}
}
