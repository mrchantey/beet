use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Logs a message when the action is run.
pub struct LogOnRun(pub Cow<'static, str>);

impl ActionMeta for LogOnRun {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl ActionSystems for LogOnRun {
	fn systems() -> SystemConfigs { log_on_run.in_set(TickSet) }
}

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
