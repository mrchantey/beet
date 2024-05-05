use crate::prelude::*;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// Runs before [`TickSet`] and In this set you can do things that need to happen before the tick.
pub struct PreTickSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// The set in which actions are run.
pub struct TickSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// Runs after [`TickSet`] and [`apply_deferred`], used to synchronize various lifecycle components
/// like [`Running`] or [`Interrupt`]
pub struct TickSyncSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
/// Runs after [`TickSyncSet`] and [`apply_deferred`].
pub struct PostTickSet;

#[derive(Debug, Clone, Default)]
// Helpers that clean up run state, this is included in the [`LifecyclePlugin`]
pub struct LifecycleSystemsPlugin;

impl Plugin for LifecycleSystemsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<BeetConfig>();
		let config = app.world().resource::<BeetConfig>();
		let schedule = config.schedule.clone();

		app /*-*/
			.configure_sets(schedule, PreTickSet)
			.configure_sets(schedule, TickSet.after(PreTickSet))
			.configure_sets(schedule, TickSyncSet.after(TickSet))
			.configure_sets(schedule, PostTickSet.after(TickSyncSet))
			.add_systems(schedule, apply_deferred.after(PreTickSet).before(TickSet))
			.add_systems(schedule, apply_deferred.after(TickSet).before(TickSyncSet))
			.add_systems(schedule, apply_deferred.after(TickSyncSet).before(PostTickSet))
			.add_systems(
				schedule,
				update_run_timers
					.run_if(|time: Option<Res<Time>>| time.is_some())
					.in_set(PreTickSet),
			)
			.add_systems(
				schedule,
				(sync_interrupts, sync_running).chain().in_set(TickSyncSet),
			)
			.add_systems(
				schedule,
				set_root_as_target_agent.in_set(PreTickSet),
			)
			/*-*/;
	}
}
