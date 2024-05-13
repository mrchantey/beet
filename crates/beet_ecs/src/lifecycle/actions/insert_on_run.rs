use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


#[derive(
	Default, PartialEq, Deref, DerefMut, Debug, Clone, Component, Reflect,
)]
#[reflect(Default, Component, ActionMeta)]
pub struct InsertOnRun<T: GenericActionComponent>(pub T);

impl<T: GenericActionComponent> ActionMeta for InsertOnRun<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Internal }
}

impl<T: GenericActionComponent> ActionSystems for InsertOnRun<T> {
	fn systems() -> SystemConfigs { insert_on_run::<T>.in_set(PreTickSet) }
}


impl<T: GenericActionComponent> InsertOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn insert_on_run<T: GenericActionComponent>(
	mut commands: Commands,
	query: Query<(Entity, &InsertOnRun<T>), Added<Running>>,
) {
	for (entity, from) in query.iter() {
		commands.entity(entity).insert(from.0.clone());
	}
}
