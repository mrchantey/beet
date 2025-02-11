use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// Mostly used for hello-world programs, logs a message when the action is run.
/// Use [`BeetDebugPlugin`] for debugging run-state.
#[action(log_on_run)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
pub struct LogOnRun(pub Cow<'static, str>);

impl Default for LogOnRun {
	fn default() -> Self { Self(Cow::Borrowed("Running...")) }
}

impl LogOnRun {
	pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
		Self(message.into())
	}
}

fn log_on_run(ev: Trigger<OnRun>, query: Query<&LogOnRun>) {
	let action = query
		.get(ev.action)
		.expect(&expect_action::to_have_action(&ev));
	log::info!("{}", action.0);
}
