use crate::prelude::*;
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use std::marker::PhantomData;
use strum::IntoEnumIterator;



#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSyncSet;
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;


pub struct ActionPlugin<
	T: IntoEnumIterator + IntoAction,
	Schedule: ScheduleLabel + Clone,
> {
	pub schedule: Schedule,
	pub phantom: PhantomData<T>,
}
impl<T: IntoEnumIterator + IntoAction> Default for ActionPlugin<T, Update> {
	fn default() -> Self {
		Self {
			schedule: Update,
			phantom: PhantomData,
		}
	}
}

impl<T: IntoEnumIterator + IntoAction, Schedule: ScheduleLabel + Clone> Plugin
	for ActionPlugin<T, Schedule>
{
	fn build(&self, app: &mut App) {
		app.configure_sets(self.schedule.clone(), PreTickSet);
		app.configure_sets(self.schedule.clone(), TickSet.after(PreTickSet));
		app.configure_sets(self.schedule.clone(), TickSyncSet.after(TickSet));
		app.configure_sets(
			self.schedule.clone(),
			PostTickSet.after(TickSyncSet),
		);

		app.add_systems(
			self.schedule.clone(),
			apply_deferred.after(PreTickSet).before(TickSet),
		);
		app.add_systems(
			self.schedule.clone(),
			apply_deferred.after(TickSet).before(TickSyncSet),
		);
		app.add_systems(
			self.schedule.clone(),
			apply_deferred.after(TickSyncSet).before(PostTickSet),
		);

		app.add_systems(
			self.schedule.clone(),
			(sync_running, sync_interrupts).in_set(TickSyncSet),
		);
		for action in T::iter().map(|item| item.into_action()) {
			app.add_systems(
				self.schedule.clone(),
				action.tick_system().in_set(TickSet),
			);
			app.add_systems(
				self.schedule.clone(),
				action.post_tick_system().in_set(TickSyncSet),
			);
		}
	}
}
