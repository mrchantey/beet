use crate::prelude::*;
use beet_core::prelude::*;

#[derive(Default)]
pub struct BeetFlowPlugin;

impl Plugin for BeetFlowPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(run_plugin::<(), (), ()>)
			.configure_sets(Update, PreTickSet)
			.configure_sets(Update, TickSet.after(PreTickSet))
			.configure_sets(Update, PostTickSet.after(TickSet))
			.add_systems(
				Update,
				// flush any triggers spawned by TriggerDeferred
				OnSpawnDeferred::flush.in_set(PreTickSet),
			);
		app.add_systems(
			Update,
			(
				tick_run_timers,
				// return_in_duration must be after tick_run_timers
				end_in_duration::<(), ()>,
			)
				.chain()
				.in_set(TickSet),
		)
		.add_observer(reset_run_time_started)
		.add_observer(reset_run_timer_stopped);
	}
}

/// Any [RunTimer] will be ticked, runs before [`TickSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PreTickSet;

/// The set in which most actions that use [`Running`] should be run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct TickSet;

/// Runs after [`TickSet`]
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct PostTickSet;



/// This plugin should be registered for any [`RunPayload`] and [`ResultPayload`] pair,
/// ensuring events are properly propagated and interrupted.
pub fn run_plugin<
	R: 'static + Send + Sync,
	T: 'static + Send + Sync,
	E: 'static + Send + Sync,
>(
	app: &mut App,
) {
	// app.add_observer(propagate_on_run::<Run>);
	app.add_observer(interrupt_on_run::<R>);
	// app.add_observer(propagate_on_result::<Result>);
	app.add_observer(interrupt_on_end::<T, E>);
}
