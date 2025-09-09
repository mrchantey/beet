use crate::prelude::*;
use bevy::prelude::*;



/// Sometimes its useful to run an action by spawning an entity,
/// for example if you want to run on the next frame to avoid
/// infinite loops or await updated world state.
/// The [`RunOnSpawn`] component will be removed immediately
/// and the [`OnRunAction`] will be triggered.
/// ## Example
/// ```
/// # use beet_flow::doctest::*;
/// # let mut world = world();
/// world.spawn(RunOnSpawn::default());
/// ```
/// ## Notes
/// This component is SparsSet as it is frequently added and removed.
#[derive(Debug, Clone, Component)]
#[component(storage = "SparseSet")]
pub struct RunOnSpawn<T = ()> {
	/// The payload of the run.
	/// By analogy if an action is a function, this would be the arguments.
	pub action: OnRunAction<T>,
}

impl<T> RunOnSpawn<T> {
	/// Create a new [`RunOnSpawn`] event, specifying the action
	pub fn new(action: OnRunAction<T>) -> Self { Self { action } }
}

impl Default for RunOnSpawn<()> {
	fn default() -> Self {
		Self {
			action: OnRun::local(),
		}
	}
}

// we use a system instead of observer to avoid infinite loops
pub(crate) fn run_on_spawn(
	mut commands: Commands,
	query: Populated<(Entity, &RunOnSpawn)>,
) {
	for (entity, run_on_spawn) in query.iter() {
		commands
			.entity(entity)
			.remove::<RunOnSpawn>()
			// we call on the entity in case it is OnRunAction::local,
			// but it still works for global actions.
			// in most cases a clone will be faster than a take
			.trigger(run_on_spawn.action.clone());
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn local() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let observers = collect_on_result(app.world_mut());
		app.world_mut().spawn((
			ReturnWith(RunResult::Success),
			RunOnSpawn::new(OnRun::local()),
		));
		observers().xpect_eq(vec![]);
		app.update();
		app.world_mut().flush();
		observers().xpect_eq(vec![("".to_string(), RunResult::Success)]);
	}
	#[test]
	fn global() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let observers = collect_on_result(app.world_mut());
		let action = app
			.world_mut()
			.spawn((Name::new("action"), ReturnWith(RunResult::Success)))
			.id();
		app.world_mut()
			.spawn(RunOnSpawn::new(OnRun::global(action)));
		observers().xpect_eq(vec![]);
		app.update();
		app.world_mut().flush();
		observers().xpect_eq(vec![("action".to_string(), RunResult::Success)]);
	}
}
