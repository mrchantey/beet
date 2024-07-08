use crate::prelude::*;
use bevy::ecs::intern::Interned;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[derive(Resource, Clone)]
pub struct BeetConfig {
	pub schedule: Interned<dyn ScheduleLabel>,
	pub set: Option<Interned<dyn SystemSet>>,
}

impl Default for BeetConfig {
	fn default() -> Self { Self::new(Update) }
}


impl BeetConfig {
	pub fn new(schedule: impl ScheduleLabel) -> Self {
		Self {
			schedule: schedule.intern(),
			set: None,
		}
	}
	pub fn add_systems<M>(
		&self,
		app: &mut App,
		systems: impl IntoSystemConfigs<M>,
	) {
		if let Some(set) = self.set {
			app.add_systems(self.schedule, systems.in_set(set));
		} else {
			app.add_systems(self.schedule, systems);
		}
	}
}
