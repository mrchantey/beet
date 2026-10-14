//! Extension methods for Bevy's schedule labels, and [`InsertSchedule`].

use crate::prelude::*;
use bevy::app::MainScheduleOrder;
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

/// Initializes a schedule and runs it every frame, beside one already in the
/// [`MainScheduleOrder`], an [`OwnedPlugin`]:
/// `app.add(InsertSchedule::before(Update, RunTests))`.
///
/// ## Panics
///
/// Panics if the anchor schedule is not in the order.
pub struct InsertSchedule<Anchor, Schedule> {
	anchor: Anchor,
	schedule: Schedule,
	after: bool,
}

impl<Anchor: ScheduleLabel, Schedule: ScheduleLabel>
	InsertSchedule<Anchor, Schedule>
{
	/// Run `schedule` directly before `anchor`.
	pub fn before(anchor: Anchor, schedule: Schedule) -> Self {
		Self {
			anchor,
			schedule,
			after: false,
		}
	}
	/// Run `schedule` directly after `anchor`.
	pub fn after(anchor: Anchor, schedule: Schedule) -> Self {
		Self {
			anchor,
			schedule,
			after: true,
		}
	}
}

impl<Anchor: ScheduleLabel, Schedule: ScheduleLabel> OwnedPlugin
	for InsertSchedule<Anchor, Schedule>
{
	fn build(self, app: &mut App) {
		// the order compares the anchor by its concrete type, so only the
		// schedule may be interned
		let schedule = self.schedule.intern();
		app.init_schedule(schedule);
		let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
		if self.after {
			order.insert_after(self.anchor, schedule);
		} else {
			order.insert_before(self.anchor, schedule);
		}
	}
}
