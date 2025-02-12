mod continue_run;
mod insert;
mod remove;
mod return_in_duration;
mod run_timer;
mod succeed_times;
use crate::prelude::*;
use bevy::prelude::*;
pub use continue_run::*;
pub use insert::*;
pub use remove::*;
pub use return_in_duration::*;
pub use run_timer::*;
pub use succeed_times::*;

pub fn continue_run_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(
			tick_run_timers.never_param_warn(),
			return_in_duration::<RunResult>.never_param_warn(),
		)
			.chain() // return_in_duration must be after tick_run_timers
			.in_set(TickSet),
	)
	.add_observer(reset_run_time_started)
	.add_observer(reset_run_timer_stopped);
}
