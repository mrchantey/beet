use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;
// use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use bevy::utils::intern::Interned;
use std::marker::PhantomData;


#[derive(Resource, Clone)]
pub struct BeetConfig {
	pub schedule: Interned<dyn ScheduleLabel>,
}

impl Default for BeetConfig {
	fn default() -> Self { Self::new(Update) }
}


impl BeetConfig {
	pub fn new(schedule: impl ScheduleLabel) -> Self {
		Self {
			schedule: schedule.intern(),
		}
	}
}

/// Plugin that adds all [`ActionSystems`] to the schedule in [`BeetConfig`], and inits the components.
#[derive(Debug, Default, Copy, Clone)]
pub struct ActionPlugin<T: 'static + Send + Sync + Bundle + ActionSystems> {
	phantom: PhantomData<T>,
}

#[cfg(feature = "reflect")]
impl<T: 'static + Send + Sync + Bundle + Reflect + ActionSystems> Plugin
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
impl<T: 'static + Send + Sync + Bundle + ActionSystems> Plugin
	for ActionPlugin<T>
{
	fn build(&self, app: &mut App) {
		let world = app.world_mut();
		world.init_bundle::<T>();

		app.init_resource::<BeetConfig>();
		let settings = app.world().resource::<BeetConfig>();
		app.add_systems(settings.schedule, T::systems());
	}
}
