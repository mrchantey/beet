mod action_observers;
mod expect;
mod request;
mod run;
mod response;
mod action_context;
pub use action_context::*;
pub use action_observers::*;
use bevy::prelude::*;
pub use expect::*;
pub use request::*;
pub use response::*;
pub use run::*;



pub fn observer_plugin(app: &mut App) {
	app.init_resource::<ActionObserverMap>();
	app.add_plugins(request_plugin::<Run>);
}
