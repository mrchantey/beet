use crate::prelude::*;
// use bevy::ecs::schedule::ScheduleLabel;
// use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
// use bevy::utils::intern::Interned;
use std::marker::PhantomData;


#[derive(Debug, Default, Copy, Clone)]
pub struct ActionPlugin<T: ActionSystems2> {
	phantom: PhantomData<T>,
}

#[cfg(feature = "reflect")]
impl<T: 'static + Send + Sync + Component + Reflect + ActionSystems2> Plugin
	for ActionPlugin<T>
where
	Self: ActionMeta,
{
	fn build(&self, app: &mut App) {
		// app.init_resource::<AppTypeRegistry>();
		let mut registry =
			app.world_mut().resource::<AppTypeRegistry>().write();
		registry.register::<T>();

		let world = app.world_mut();
		world.init_component::<T>();

		app.init_resource::<BeetConfig>();
		let settings = app.world().resource::<BeetConfig>();
		app.add_systems(settings.schedule, T::system());
	}
}

#[cfg(not(feature = "reflect"))]
impl<T: 'static + Send + Sync + Component + ActionSystems2> Plugin
	for ActionPlugin<T>
where
	Self: ActionMeta,
{
	fn build(&self, app: &mut App) {
		let world = app.world_mut();
		world.init_component::<T>();

		app.init_resource::<BeetConfig>();
		let settings = app.world().resource::<BeetConfig>();
		app.add_systems(settings.schedule, T::systems());
	}
}
