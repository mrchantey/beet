mod action_observers;
mod expect;
mod on_result;
mod on_run;
pub use action_observers::*;
use bevy::prelude::*;
pub use expect::*;
pub use on_result::*;
pub use on_run::*;


pub fn observer_plugin(app: &mut App) {
	app.init_resource::<ActionObserverMap>();
	app.add_plugins(run_plugin::<(), RunResult>);
}
