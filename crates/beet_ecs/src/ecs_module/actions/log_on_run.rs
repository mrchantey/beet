use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

#[derive_action(Default)]
#[action(graph_role=GraphRole::Node)]
/// Logs a message when the action is run.
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
		println!("{}", log.0)
	}
}

#[derive_action]
#[action(graph_role=GraphRole::Node)]
/// Logs the [`Name`] when the action is run.
pub struct LogNameOnRun;

fn log_name_on_run(query: Query<&Name, (With<LogNameOnRun>, Added<Running>)>) {
	for name in query.iter() {
		log::info!("Running: {name}")
	}
}
