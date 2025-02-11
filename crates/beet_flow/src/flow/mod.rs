mod action_event;
mod action_observers;
mod continue_run;
mod expect;
mod on_result;
mod on_run;
mod run_on_spawn;
use crate::prelude::*;
pub use action_event::*;
pub use action_observers::*;
use bevy::prelude::*;
pub use continue_run::*;
pub use expect::*;
pub use on_result::*;
pub use on_run::*;
pub use run_on_spawn::*;


pub fn observer_plugin(app: &mut App) {
	app.init_resource::<ActionObserverMap>()
		.add_plugins((
			run_plugin::<(), RunResult>,
			run_plugin::<RequestScore, ScoreValue>,
		))
		.add_systems(
			Update,
			(
				run_on_spawn.never_param_warn(),
				tick_run_timers.never_param_warn(),
			)
				.in_set(BeetTickSet),
		)
		.add_observer(reset_run_time_started)
		.add_observer(reset_run_timer_stopped);
}

/// All `beet_flow` systems are run on the Update schedule in this set.
#[derive(Debug, Clone, PartialEq, Eq, Hash, SystemSet)]
pub struct BeetTickSet;



pub fn run_plugin<Run: RunPayload, Result: ResultPayload>(app: &mut App) {
	app.add_observer(propagate_on_run::<Run>);
	app.add_observer(propagate_on_result::<Result>);
}


