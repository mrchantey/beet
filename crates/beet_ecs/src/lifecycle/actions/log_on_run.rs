use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// Mostly used for hello-world programs, logs a message when the action is run.
/// Use [`BeetDebugPlugin`] for debugging run-state.
#[derive(Debug, Clone, PartialEq, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[systems(log_on_run.in_set(TickSet))]
pub struct LogOnRun(pub Cow<'static, str>);

impl Default for LogOnRun {
	fn default() -> Self { Self(Cow::Borrowed("Running...")) }
}

impl LogOnRun {
	pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
		Self(message.into())
	}
}

fn log_on_run(query: Query<&LogOnRun, Added<Running>>) {
	for log in query.iter() {
		log::info!("{}", log.0)
	}
}
