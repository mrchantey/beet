use crate::prelude::*;
use bevy::prelude::*;


/// This will add the [`Running`] component to the behavior when [`OnRun`] is triggered,
/// and remove it when [`OnRunResult`] is triggered.
///
/// This should be added as a required component on any action that has a `With<Running>` query filter,
/// not added to behaviors directly, because its easy to forget.
///
/// Actions with long running systems may look like this:
/// ```
///	# use bevy::prelude::*;
///	# use beet_flow::prelude::*;
///
///
/// #[derive(Component, Action)]
/// #[systems(my_long_action)]
/// #[require(ContinueRun)]
/// struct MyLongAction;
///
/// fn my_long_action(query: Query<&MyLongAction, With<Running>>){
///
/// for action in query.iter(){
///   // etc.
/// }
///
/// }
/// ```
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer, Insert<OnRun,Running>, Remove<OnResult, Running>)]
pub struct ContinueRun;




/// Indicate this node is currently long-running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component, Default)]
pub struct Running;


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn adds() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		// adds
		let entity =
			world.spawn(ContinueRun).flush_trigger(OnRun::local()).id();
		expect(world.get::<Running>(entity)).to_be_some();
	}
	#[test]
	fn removes() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let entity = world.spawn((Running, ContinueRun)).id();
		world.flush_trigger(OnResultAction::global(entity, RunResult::Success));
		expect(world.get::<Running>(entity)).to_be_none();
	}
}
