use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Sets a component when this behavior starts running.
/// This does nothing if the entity does not have the component.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct SetOnRun<T: GenericActionComponent>(pub T);

impl<T: Default + GenericActionComponent> Default for SetOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionComponent> ActionMeta for SetOnRun<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<T: GenericActionComponent> ActionSystems for SetOnRun<T> {
	fn systems() -> SystemConfigs { set_on_run::<T>.in_set(PostTickSet) }
}


impl<T: GenericActionComponent> SetOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_on_run<T: GenericActionComponent>(
	mut query: Query<(&SetOnRun<T>, &mut T), Added<Running>>,
) {
	for (from, mut to) in query.iter_mut() {
		*to = from.0.clone();
	}
}
