mod continue_run;
mod insert;
mod remove;
mod run_timer;
mod succeed_times;
mod return_in_duration;
use crate::prelude::*;
use bevy::prelude::*;
pub use continue_run::*;
pub use insert::*;
pub use remove::*;
pub use run_timer::*;
pub use succeed_times::*;
pub use return_in_duration::*;

pub fn continue_run_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(
			return_in_duration::<RunResult>.never_param_warn(),
			tick_run_timers.never_param_warn(),
		)
			.in_set(BeetTickSet),
	)
	.add_observer(reset_run_time_started)
	.add_observer(reset_run_timer_stopped);
}
