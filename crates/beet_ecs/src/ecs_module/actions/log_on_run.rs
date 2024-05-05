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
		println!("{}", log.0)
	}
}

#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
/// Logs the [`Name`] when the action is run.
pub struct LogNameOnRun;

impl ActionMeta for LogNameOnRun {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl ActionSystems for LogNameOnRun {
	fn systems() -> SystemConfigs { log_name_on_run.in_set(TickSet) }
}

fn log_name_on_run(query: Query<&Name, (With<LogNameOnRun>, Added<Running>)>) {
	for name in query.iter() {
		log::info!("Running: {name}")
	}
}
