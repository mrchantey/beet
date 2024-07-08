use crate::prelude::*;
use bevy::prelude::*;

/// Triggers the given event when this behavior starts running.
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[systems(trigger_on_run::<T>.in_set(TickSet))]
pub struct TriggerOnRun<T: GenericActionEvent>(pub T);

impl<T: Default + GenericActionEvent> Default for TriggerOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionEvent> TriggerOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn trigger_on_run<T: GenericActionEvent>(
	mut writer: EventWriter<T>,
	query: Query<&TriggerOnRun<T>, Added<Running>>,
) {
	for trigger in query.iter() {
		writer.send(trigger.0.clone());
	}
}
