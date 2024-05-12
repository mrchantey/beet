use crate::prelude::*;
use bevy::prelude::*;


/// A plugin that logs lifecycle events for behaviors with a [`Name`].
pub struct BeetDebugPlugin {
	log_on_start: bool,
	log_on_update: bool,
	log_on_stop: bool,
}

impl Default for BeetDebugPlugin {
	fn default() -> Self {
		Self {
			log_on_start: true,
			log_on_update: false,
			log_on_stop: false,
		}
	}
}


impl Plugin for BeetDebugPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BeetConfig>();
		let config = app.world().resource::<BeetConfig>();
		let schedule = config.schedule.clone();


		if self.log_on_start {
			app.add_systems(schedule, log_on_running_added.in_set(PostTickSet));
		}
		if self.log_on_update {
			app.add_systems(
				schedule,
				log_on_running
					.after(log_on_running_added)
					.in_set(PostTickSet),
			);
		}
		if self.log_on_stop {
			app.add_systems(
				schedule,
				log_on_running_removed.after(PostTickSet),
			);
		}
	}
}
fn log_on_running_added(query: Query<&Name, Added<Running>>) {
	for name in query.iter() {
		log::info!("Started: {name}")
	}
}
fn log_on_running(query: Query<&Name, With<Running>>) {
	for name in query.iter() {
		log::info!("Running: {name}")
	}
}
fn log_on_running_removed(
	query: Query<&Name>,
	mut removed: RemovedComponents<Running>,
) {
	for removed in removed.read() {
		if let Ok(name) = query.get(removed) {
			log::info!("Stopped: {name}")
		}
	}
}
