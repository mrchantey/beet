use bevy::app::MainScheduleOrder;
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


	/// Register this schedule in the main schedule order after the specified schedule
	/// # Panics
	/// Panics if the other schedule has not been registered yet.
	fn register_before(
		self,
		app: &mut App,
		before: impl Clone + ScheduleLabel,
	) {
		app.init_schedule(self.clone());
		app.init_schedule(before.clone());
		let mut main_schedule_order =
			app.world_mut().resource_mut::<MainScheduleOrder>();
		main_schedule_order.insert_before(before, self);
	}
	/// Register this schedule in the main schedule order after the specified schedule
	/// # Panics
	/// Panics if the other schedule has not been registered yet.
	fn register_after(self, app: &mut App, after: impl Clone + ScheduleLabel) {
		app.init_schedule(self.clone());
		app.init_schedule(after.clone());
		let mut main_schedule_order =
			app.world_mut().resource_mut::<MainScheduleOrder>();
		main_schedule_order.insert_after(after, self);
	}
}
