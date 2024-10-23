use crate::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;


/// Config for logging the lifecycle of behaviors, see [`BeetDebugPlugin`].
#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct BeetDebugConfig {
	pub log_on_start: bool,
	pub log_on_update: bool,
	pub log_on_stop: bool,
	pub log_to_stdout: bool,
}

impl Default for BeetDebugConfig {
	fn default() -> Self {
		Self {
			log_on_start: true,
			log_on_update: false,
			log_on_stop: false,
			log_to_stdout: true,
		}
	}
}

impl BeetDebugConfig {
	pub fn start_and_stop() -> Self {
		Self {
			log_on_stop: true,
			..default()
		}
	}
}

/// A plugin that logs lifecycle events for behaviors with a [`Name`].
/// It triggers [OnLogMessage] events, and also adds a listener that
/// will print to stdout if [`BeetDebugConfig::log_to_stdout`] is true.
#[derive(Clone)]
pub struct BeetDebugPlugin;
impl Plugin for BeetDebugPlugin {
	fn build(&self, app: &mut App) {
		app.add_observer(log_on_start)
			.add_observer(log_on_stop)
			.add_systems(Update, log_log_on_run)
			.init_resource::<BeetConfig>()
			.register_type::<BeetDebugConfig>();

		let config = app.world().resource::<BeetConfig>();
		let schedule = config.schedule.clone();
		app.add_systems(
		schedule,
			log_on_update.run_if(
				|config: Option<Res<BeetDebugConfig>>| {
					config.map(|c| c.log_on_update).unwrap_or_default()
				},
			)
			.in_set(PostTickSet)
	)
		.add_observer(
			(|trigger: Trigger<OnLogMessage>,config:Res<BeetDebugConfig>| {
				if config.log_to_stdout {
				log::info!("{}", **trigger.event());
				}
			}).never_param_warn(),
		)
		/*-*/;
	}
}

fn log_log_on_run(
	mut commands: Commands,
	query: Query<&LogOnRun, Added<Running>>,
) {
	for log in query.iter() {
		let msg = format!("LogOnRun: {}", log.0.to_string());
		commands.trigger(OnLogMessage::new(msg));
	}
}

fn log_on_start(
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	config: Option<Res<BeetDebugConfig>>,
	query: Query<&Name>,
) {
	// TODO run_if https://github.com/bevyengine/bevy/issues/14157
	if !config.map(|c| c.log_on_start).unwrap_or_default() {
		return;
	}
	let msg = query
		.get(trigger.entity())
		.map(|n| format!("Started: {n}"))
		.unwrap_or_else(|_| format!("Started: {}", trigger.entity()));
	commands.trigger(OnLogMessage::new(msg));
}

fn log_on_update(mut commands: Commands, query: Query<&Name, With<Running>>) {
	for name in query.iter() {
		let msg = format!("Continue: {name}");
		commands.trigger(OnLogMessage::new(msg));
	}
}

fn log_on_stop(
	trigger: Trigger<OnRunResult>,
	config: Option<Res<BeetDebugConfig>>,
	query: Query<&Name>,
	mut commands: Commands,
) {
	// TODO run_if https://github.com/bevyengine/bevy/issues/14157
	if !config.map(|c| c.log_on_stop).unwrap_or_default() {
		return;
	}
	let msg = query
		.get(trigger.entity())
		.map(|n| format!("Stopped: {n}"))
		.unwrap_or_else(|_| format!("Stopped: {}", trigger.entity()));
	commands.trigger(OnLogMessage::new(msg));
}
