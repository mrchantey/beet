use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;



#[extend::ext(name=ScheduleLabelExt)]
pub impl<T: Default + ScheduleLabel> T {
	/// Convert a [`ScheduleLabel`] into an exclusive system, running it once.
	fn run() -> impl Fn(&mut World) {
		move |world: &mut World| {
			world.run_schedule(T::default());
		}
	}
}
