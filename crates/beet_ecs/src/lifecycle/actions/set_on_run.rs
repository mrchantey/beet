use crate::prelude::*;
use bevy::prelude::*;

/// Sets a component when this behavior starts running.
/// This does nothing if the entity does not have the component.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(set_on_run::<T>.in_set(PostTickSet))]
pub struct SetOnRun<T: GenericActionComponent>(pub T);

impl<T: Default + GenericActionComponent> Default for SetOnRun<T> {
	fn default() -> Self { Self(T::default()) }
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
