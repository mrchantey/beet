use crate::prelude::*;
use beet_core::prelude::*;
use std::borrow::Cow;

/// Logs a provided message when it runs, useful for debugging.
/// ## Tags
/// - [InputOutput](ActionTag::InputOutput)
/// # Example
/// ```
/// # use beet_core::prelude::*;
/// # use beet_flow::prelude::*;
/// # let mut world = BeetFlowPlugin::world();
/// world
///		.spawn(LogOnRun::new("Running..."))
///		.trigger_target(GetOutcome);
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

fn log_on_run(ev: On<GetOutcome>, query: Query<&LogOnRun>) -> Result {
	let action = query
		.get(ev.event_target())
		.expect(&expect_action::to_have_action(&ev));
	info!("{}", action.0);
	Ok(())
}
