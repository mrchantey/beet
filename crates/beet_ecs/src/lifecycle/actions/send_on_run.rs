use crate::prelude::*;
use bevy::prelude::*;

/// Sends the given event when this behavior starts running.
#[derive(Debug, Clone, PartialEq, Deref, DerefMut, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[observers(send_on_run::<T>)]
pub struct SendOnRun<T: GenericActionEvent>(pub T);

impl<T: Default + GenericActionEvent> Default for SendOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionEvent> SendOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn send_on_run<T: GenericActionEvent>(
	trigger: Trigger<OnRun>,
	mut writer: EventWriter<T>,
	query: Query<&SendOnRun<T>>,
) {
	let action = query
		.get(trigger.entity())
		.expect(expect_action::NO_ACTION_COMP);
	writer.send(action.0.clone());
}
