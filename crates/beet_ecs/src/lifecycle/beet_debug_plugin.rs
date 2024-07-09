use crate::prelude::*;
use bevy::prelude::*;
use std::borrow::Cow;
use std::marker::PhantomData;


#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub struct BeetDebugConfig {
	pub log_on_start: bool,
	pub log_on_update: bool,
	pub log_on_stop: bool,
}

impl Default for BeetDebugConfig {
	fn default() -> Self {
		Self {
			log_on_start: true,
			log_on_update: false,
			log_on_stop: false,
		}
	}
}

pub struct BeetDebugPluginBase;
impl Plugin for BeetDebugPluginBase {
	fn build(&self, app: &mut App) {
		app.observe(log_on_start_observer)
			.observe(log_on_stop_observer);
	}
}


pub struct BeetDebugPluginStdout;
impl Plugin for BeetDebugPluginStdout {
	fn build(&self, app: &mut App) {
		app
		.add_plugins(BeetDebugPlugin::new(log_stdout))
		.observe(
			|trigger: Trigger<OnLogMessage>| {
				log::info!("{}", **trigger.event());
			},
		)
		/*-*/;
	}
}
fn log_stdout(In(messages): In<Vec<String>>) {
	for message in messages.into_iter() {
		log::info!("{message}");
	}
}

/// A plugin that logs lifecycle events for behaviors with a [`Name`].
pub struct BeetDebugPlugin<
	M: 'static + Send + Sync,
	T: 'static + Send + Sync + Clone + IntoSystem<Vec<String>, (), M>,
> {
	log_system: T,
	_marker: PhantomData<M>,
}

impl<
		M: 'static + Send + Sync,
		T: 'static + Send + Sync + Clone + IntoSystem<Vec<String>, (), M>,
	> BeetDebugPlugin<M, T>
{
	pub fn new(log_system: T) -> Self {
		Self {
			log_system,
			_marker: PhantomData,
		}
	}
}

impl<
		M: 'static + Send + Sync,
		T: 'static + Send + Sync + Clone + IntoSystem<Vec<String>, (), M>,
	> Plugin for BeetDebugPlugin<M, T>
{
	fn build(&self, app: &mut App) {
		app.init_resource::<BeetConfig>()
			.register_type::<BeetDebugConfig>();
		let config = app.world().resource::<BeetConfig>();
		let schedule = config.schedule.clone();

		app.add_systems(
			schedule,
			(
				log_on_start.pipe(self.log_system.clone()).run_if(
					|config: Option<Res<BeetDebugConfig>>| {
						config.map(|c| c.log_on_start).unwrap_or_default()
					},
				),
				log_on_update.pipe(self.log_system.clone()).run_if(
					|config: Option<Res<BeetDebugConfig>>| {
						config.map(|c| c.log_on_update).unwrap_or_default()
					},
				),
			)
				.chain()
				.in_set(PostTickSet),
		)
		.add_systems(
			schedule,
			log_on_stop
				.pipe(self.log_system.clone())
				.after(PostTickSet)
				.run_if(|config: Option<Res<BeetDebugConfig>>| {
					config.map(|c| c.log_on_stop).unwrap_or_default()
				}),
		)
		/*-*/;
	}
}


#[deprecated = "use observers"]
fn log_on_start(query: Query<&Name, Added<Running>>) -> Vec<String> {
	query
		.iter()
		.map(|name| format!("Started: {name}"))
		.collect()
}
fn log_on_update(query: Query<&Name, With<Running>>) -> Vec<String> {
	query
		.iter()
		.map(|name| format!("Running: {name}"))
		.collect()
}
#[deprecated = "use observers"]
fn log_on_stop(
	query: Query<&Name>,
	mut removed: RemovedComponents<Running>,
) -> Vec<String> {
	removed
		.read()
		.filter_map(|removed| query.get(removed).ok())
		.map(|name| format!("Stopped: {name}"))
		.collect()
}


#[derive(Event, Deref)]
pub struct OnLogMessage(pub Cow<'static, str>);

impl OnLogMessage {
	pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
		Self(message.into())
	}
}
fn log_on_start_observer(
	trigger: Trigger<OnRun>,
	config: Option<Res<BeetDebugConfig>>,
	query: Query<&Name>,
	mut commands: Commands,
) {
	// TODO run_if https://github.com/bevyengine/bevy/issues/14157
	if !config.map(|c| c.log_on_start).unwrap_or_default() {
		return;
	}
	let name = query
		.get(trigger.entity())
		.map(|n| format!("Started: {n}"))
		.unwrap_or_else(|_| format!("Started: {}", trigger.entity()));
	commands.trigger(OnLogMessage::new(name));
}
fn log_on_stop_observer(
	trigger: Trigger<OnRunResult>,
	config: Option<Res<BeetDebugConfig>>,
	query: Query<&Name>,
	mut commands: Commands,
) {
	// TODO run_if https://github.com/bevyengine/bevy/issues/14157
	if !config.map(|c| c.log_on_stop).unwrap_or_default() {
		return;
	}
	let name = query
		.get(trigger.entity())
		.map(|n| format!("Stopped: {n}"))
		.unwrap_or_else(|_| format!("Stopped: {}", trigger.entity()));
	commands.trigger(OnLogMessage::new(name));
}
