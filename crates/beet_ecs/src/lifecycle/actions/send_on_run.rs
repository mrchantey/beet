use crate::prelude::*;
use bevy::prelude::*;

/// Sends the given event when this behavior starts running.
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[systems(send_on_run::<T>.in_set(TickSet))]
pub struct SendOnRun<T: GenericActionEvent>(pub T);

impl<T: Default + GenericActionEvent> Default for SendOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionEvent> SendOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn send_on_run<T: GenericActionEvent>(
	mut writer: EventWriter<T>,
	query: Query<&SendOnRun<T>, Added<Running>>,
) {
	for trigger in query.iter() {
		writer.send(trigger.0.clone());
	}
}
