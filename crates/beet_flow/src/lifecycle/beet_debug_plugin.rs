use super::PostTickSet;
use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;

/// An 'stdout observer', triggering this will log to the ui terminal.
#[derive(Debug, Event, Deref)]
pub struct OnLogMessage(pub Cow<'static, str>);

impl OnLogMessage {
	pub fn new(msg: impl Into<Cow<'static, str>>) -> Self { Self(msg.into()) }
	pub fn new_with_query(
		entity: Entity,
		query: &Query<&Name>,
		prefix: &str,
	) -> Self {
		Self::new_with_optional(entity, query.get(entity).ok(), prefix)
	}
	pub fn new_with_optional(
		entity: Entity,
		name: Option<&Name>,
		prefix: &str,
	) -> Self {
		let msg = name
			.map(|n| format!("{prefix}: {n}"))
			.unwrap_or_else(|| format!("{prefix}: {entity}"));
		Self(msg.into())
	}
}

/// marker resource for [log_on_run]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct LogOnRunMarker;
/// marker resource for [log_on_run_result]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct LogOnRunResultMarker;
/// marker resource for [log_running]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct LogRunningMarker;
/// marker resource for [log_to_stdout]
#[derive(Debug, Default, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct LogToStdoutMarker;

/// A plugin that logs lifecycle events for behaviors with a [`Name`].
/// It triggers [OnLogMessage] events, and also adds a listener that
/// will print to stdout if [`BeetDebugConfig::log_to_stdout`] is true.
///
/// If a [`BeetDebugConfig`] is not present, it will use the default.
#[derive(Debug, Default, Clone)]
pub struct BeetDebugPlugin {
	pub log_on_run: bool,
	pub log_running: bool,
	pub log_on_run_result: bool,
	/// disable logging to stdout, useful if instead using rendered terminal,
	/// networking etc.
	pub no_stdout: bool,
}
impl Plugin for BeetDebugPlugin {
	fn build(&self, app: &mut App) {
		let config = app.world().resource::<BeetConfig>();
		let schedule = config.schedule.clone();
		// TODO when resolved: [Observers::run_if](https://github.com/bevyengine/bevy/issues/14195)
		app.add_observer(log_on_run.never_param_warn())
			.add_observer(log_on_run_result.never_param_warn())
			.add_observer(log_to_stdout.never_param_warn())
			.add_systems(
				schedule,
				log_running
					.never_param_warn()
					.run_if(resource_exists::<LogRunningMarker>)
					.in_set(PostTickSet),
			);

		if self.log_on_run {
			app.init_resource::<LogOnRunMarker>();
		}

		if self.log_on_run_result {
			app.init_resource::<LogOnRunResultMarker>();
		}

		if self.log_running {
			app.init_resource::<LogRunningMarker>();
		}

		if !self.no_stdout {
			app.init_resource::<LogToStdoutMarker>();
		}
	}
}


fn log_to_stdout(trigger: Trigger<OnLogMessage>, _m: Res<LogToStdoutMarker>) {
	log::info!("{}", **trigger.event());
}

/// we use
fn log_on_run(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<&Name>,
	_m: Res<LogOnRunMarker>,
) {
	commands.trigger(OnLogMessage::new_with_query(
		trigger.entity(),
		&query,
		"OnRun",
	));
}


fn log_on_run_result(
	trigger: Trigger<OnRunResult>,
	mut commands: Commands,
	query: Query<&Name>,
	_m: Res<LogOnRunResultMarker>,
) {
	commands.trigger(OnLogMessage::new_with_query(
		trigger.entity(),
		&query,
		&format!("{:?}", &*trigger),
	));
}

fn log_running(
	mut commands: Commands,
	query: Populated<(Entity, Option<&Name>), With<Running>>,
) {
	for (entity, name) in query.iter() {
		let name = name
			.map(|n| n.to_string())
			.unwrap_or_else(|| entity.to_string());
		let msg = format!("Running: {}", name);
		commands.trigger(OnLogMessage::new(msg));
	}
}
