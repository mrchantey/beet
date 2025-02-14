//! A module for long running actions.
//! The core of long running actions in Beet is
//! systems that filter by the [Running] component.
/// For usage see the [Running] component.
mod continue_run;
mod insert;
mod remove;
mod return_in_duration;
mod run_timer;
use crate::prelude::*;
use bevy::prelude::*;
pub use continue_run::*;
pub use insert::*;
pub use remove::*;
pub use return_in_duration::*;
pub use run_timer::*;


/// Registers systems and observers required for long running actions.
pub fn continue_run_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(
			tick_run_timers.never_param_warn(),
			// return_in_duration must be after tick_run_timers
			return_in_duration::<RunResult>.never_param_warn(),
		)
			.chain()
			.in_set(TickSet),
	)
	.add_observer(reset_run_time_started)
	.add_observer(reset_run_timer_stopped);
}
