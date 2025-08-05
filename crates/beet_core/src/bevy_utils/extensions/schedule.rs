use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;



#[extend::ext(name=ScheduleLabelExt)]
pub impl<T: Clone + ScheduleLabel> T {
	/// Convert a [`ScheduleLabel`] into an exclusive system, running it once.
	fn run(self) -> impl Fn(&mut World) {
		move |world: &mut World| {
			world.run_schedule(self.clone());
		}
	}
}
