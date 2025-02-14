//! The core functionality of beet_flow, this module primarily
//! handles routing [`OnRun`] and [`OnResult`] between each
//! [`ActionEntity`] and any corresponding [`ActionObserver`].
mod action_event;
mod action_observers;
mod beet_debug_plugin;
pub mod expect_action;
mod on_result;
mod on_run;
mod run_on_spawn;
use crate::prelude::*;
pub use action_event::*;
pub use action_observers::*;
pub use beet_debug_plugin::*;
use bevy::prelude::*;
pub use on_result::*;
pub use on_run::*;
pub use run_on_spawn::*;
mod interrupt_on_result;
mod interrupt_on_run;
use interrupt_on_result::*;
pub use interrupt_on_run::*;
use std::fmt::Debug;

/// Sets up the base functionality for [`OnRun`] and [`OnResult`] routing.
pub(crate) fn control_flow_plugin(app: &mut App) {
	app.init_resource::<ActionObserverMap>()
		.add_plugins((
			run_plugin::<(), RunResult>,
			run_plugin::<RequestScore, ScoreValue>,
		))
		.configure_sets(Update, PreTickSet)
		.configure_sets(Update, TickSet.after(PreTickSet))
		.configure_sets(Update, PostTickSet.after(TickSet))
		.add_systems(
			Update,
			run_on_spawn.never_param_warn().in_set(PreTickSet),
		);
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
pub fn run_plugin<Run: RunPayload, Result: ResultPayload>(app: &mut App) {
	app.add_observer(propagate_on_run::<Run>);
	app.add_observer(interrupt_on_run::<Run>);
	app.add_observer(propagate_on_result::<Result>);
	app.add_observer(interrupt_on_result::<Result>);
}


/// Every [RunPayload] must have a corresponding [ResultPayload],
/// a clear example of this is the [RequestScore] and [ScoreValue] pair.
pub trait RunPayload: 'static + Send + Sync + Clone + Debug {
	/// Specifies the [ResultPayload] that corresponds to this [RunPayload].
	type Result: ResultPayload<Run = Self>;
}
/// Every [ResultPayload] must have a corresponding [RunPayload],
/// a clear example of this is the [RequestScore] and [ScoreValue] pair.
pub trait ResultPayload: 'static + Send + Sync + Clone + Debug {
	/// Specifies the [RunPayload] that corresponds to this [ResultPayload].
	type Run: RunPayload<Result = Self>;
}

impl RunPayload for () {
	type Result = RunResult;
}
impl ResultPayload for RunResult {
	type Run = ();
}
