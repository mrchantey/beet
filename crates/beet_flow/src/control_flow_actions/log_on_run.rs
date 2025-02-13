use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// ## Tags
/// - [InputOutput](ActionTag::InputOutput)
/// 
/// Can be used for debugging, logs a provided message when it runs.
/// ```
/// # use bevy::prelude::*;
/// # use beet_flow::prelude::*;
/// World::new()
///		.spawn(LogOnRun::new("Running..."))
///		.flush_trigger(OnRun::local());
/// ```
#[action(log_on_run)]
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
pub struct LogOnRun(pub Cow<'static, str>);

impl LogOnRun {
	/// Create a new [`LogOnRun`] action with the provided message.
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
