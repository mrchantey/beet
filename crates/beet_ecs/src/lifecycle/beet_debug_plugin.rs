use crate::prelude::*;
use bevy::prelude::*;
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

pub struct BeetDebugPluginStdout;
impl Plugin for BeetDebugPluginStdout {
	fn build(&self, app: &mut App) {
		app.add_plugins(BeetDebugPlugin::new(log_stdout));
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
		// .add_systems(
		// 	schedule,
		// 	,
		// )
		.add_systems(
			schedule,
			log_on_stop
				.pipe(self.log_system.clone())
				.after(PostTickSet)
				.run_if(|config: Option<Res<BeetDebugConfig>>| {
					config.map(|c| c.log_on_stop).unwrap_or_default()
				}),
		);
	}
}



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
