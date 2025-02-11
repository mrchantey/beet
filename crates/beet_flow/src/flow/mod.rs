mod action_observers;
mod expect;
mod on_result;
mod on_run;
mod run_on_spawn;
use crate::prelude::*;
pub use action_observers::*;
use bevy::prelude::*;
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
		.add_systems(Update, run_on_spawn);
}


pub fn run_plugin<Run: RunPayload, Result: ResultPayload>(app: &mut App) {
	app.add_observer(run_action_observers::<Run>);
	app.add_observer(run_child_result_observers::<Result>);
}
