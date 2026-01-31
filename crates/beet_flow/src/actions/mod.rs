//! Built-in actions for controlling behavior tree execution.
//!
//! Actions are components that respond to [`GetOutcome`] events and eventually
//! trigger an [`Outcome`]. This module provides common control flow patterns:
//!
//! ## Control Flow Actions
//!
//! - [`Sequence`]: Runs children in order until one fails
//! - [`Fallback`]: Runs children in order until one succeeds
//! - [`InfallibleSequence`]: Runs all children regardless of results
//! - [`Parallel`]: Runs all children simultaneously
//! - [`HighestScore`]: Runs the highest-scoring child (Utility AI)
//!
//! ## Lifecycle Actions
//!
//! - [`EndWith`]: Immediately returns a specified value
//! - [`EndInDuration`]: Returns after a duration elapses
//! - [`InsertOn`] / [`RemoveOn`]: Insert or remove components on events
//! - [`Retrigger`]: Re-runs an action based on its result
//!
//! ## Utility Actions
//!
//! - [`LogOnRun`] / [`LogNameOnRun`]: Debug logging
//! - [`ExitOnEnd`] / [`ExitOnFail`]: Convert outcomes to app exit
//! - [`RunNext`]: Chain actions together (state machine pattern)
//! - [`AwaitReady`]: Wait for async initialization
//!
//! If you think a missing action should be built-in, please open an issue.
//!
//! [`GetOutcome`]: crate::prelude::GetOutcome
//! [`Outcome`]: crate::prelude::Outcome
mod interrupt_on;
pub use interrupt_on::*;
mod await_ready;
mod exit_on_end;
mod exit_on_fail;
mod infallible_sequence;
mod loop_times;
mod trigger_deferred;
pub use await_ready::*;
pub use infallible_sequence::*;
pub use loop_times::*;
pub use trigger_deferred::*;
mod end_with;
pub use end_with::*;
pub use exit_on_end::*;
pub use exit_on_fail::*;
mod end_in_duration;
pub use end_in_duration::*;
mod insert_on;
pub use insert_on::*;
mod remove_on;
pub use remove_on::*;
mod fallback;
mod highest_score;
mod log_name_on_run;
mod log_on_run;
mod parallel;
mod retrigger;
mod run_next;
mod sequence;
pub use fallback::*;
pub use highest_score::*;
pub use log_name_on_run::*;
pub use log_on_run::*;
pub use parallel::*;
pub use retrigger::*;
pub use run_next::*;
pub use sequence::*;
mod succeed_times;
pub use succeed_times::*;
#[cfg(feature = "bevy_default")]
mod run_on_asset_ready;
#[cfg(feature = "bevy_default")]
pub use run_on_asset_ready::*;
