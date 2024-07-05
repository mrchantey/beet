use crate::prelude::*;
use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
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
#[derive(Debug, Copy, Clone)]
pub struct ActionPlugin<T: 'static + Send + Sync + Bundle + ActionSystems> {
	phantom: PhantomData<T>,
}

impl<T: 'static + Send + Sync + Bundle + ActionSystems> Default
	for ActionPlugin<T>
{
	fn default() -> Self {
		Self {
			phantom: Default::default(),
		}
	}
}

#[cfg(feature = "reflect")]
impl<
		T: 'static
			+ Send
			+ Sync
			+ Bundle
			+ Reflect
			+ bevy::reflect::GetTypeRegistration
			+ ActionSystems,
	> Plugin for ActionPlugin<T>
// where
// 	Self: ActionMeta,
{
	fn build(&self, app: &mut App) {
		app.register_type::<T>();
		build_common::<T>(app);
	}
}

#[cfg(not(feature = "reflect"))]
impl<T: 'static + Send + Sync + Bundle + ActionSystems> Plugin
	for ActionPlugin<T>
{
	fn build(&self, app: &mut App) { build_common::<T>(app); }
}

fn build_common<T: 'static + Send + Sync + Bundle + ActionSystems>(
	app: &mut App,
) {
	let world = app.world_mut();
	world.init_bundle::<T>();

	app.init_resource::<BeetConfig>();
	let settings = app.world().resource::<BeetConfig>();
	app.add_systems(settings.schedule, T::systems());
}
