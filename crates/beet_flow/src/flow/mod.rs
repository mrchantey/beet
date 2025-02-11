mod action_observers;
mod expect;
mod on_result;
mod on_run;
use crate::prelude::*;
pub use action_observers::*;
use bevy::prelude::*;
pub use expect::*;
pub use on_result::*;
pub use on_run::*;


pub fn observer_plugin(app: &mut App) {
	app.init_resource::<ActionObserverMap>();
	app.add_plugins(run_plugin::<(), RunResult>);
	app.add_plugins(run_plugin::<RequestScore, ScoreValue>);
}


pub fn run_plugin<Run: RunPayload, Result: ResultPayload>(app: &mut App) {
	app.add_observer(run_action_observers::<Run>);
	app.add_observer(trigger_result_on_parent_observers::<Result>);
}
