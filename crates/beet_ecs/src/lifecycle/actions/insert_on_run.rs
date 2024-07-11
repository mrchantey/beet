use crate::prelude::*;
use bevy::prelude::*;

/// Inserts the given component when this behavior starts running.
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(insert_on_run::<T>.in_set(PreTickSet))]
pub struct InsertOnRun<T: GenericActionComponent>(pub T);

impl<T: Default + GenericActionComponent> Default for InsertOnRun<T> {
	fn default() -> Self { Self(T::default()) }
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
