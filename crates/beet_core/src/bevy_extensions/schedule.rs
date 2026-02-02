//! Extension methods for Bevy's schedule labels.

use crate::prelude::*;
use bevy::ecs::schedule::ScheduleLabel;

/// Extension trait adding utility methods to schedule labels.
#[extend::ext(name=ScheduleLabelExt)]
pub impl<T: Default + ScheduleLabel> T {
	/// Converts a [`ScheduleLabel`] into an exclusive system that runs the schedule once.
	fn as_system() -> impl Fn(&mut World) {
		move |world: &mut World| {
			world.run_schedule(T::default());
		}
	}
}
