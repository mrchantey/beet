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
#[require(RunTimer, InsertOnRun<Running>, RemoveOnRunResult<Running>)]
pub struct ContinueRun;

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use ::sweet::prelude::*;

	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<(
			InsertOnRun<Running>,
			RemoveOnRunResult<Running>,
		)>::default());
		let world = app.world_mut();

		let entity = world.spawn(ContinueRun).flush_trigger(OnRun).id();
		expect(world.entities().len()).to_be(3);
		expect(&*world).to_have_component::<Running>(entity);
		world
			.entity_mut(entity)
			.flush_trigger(OnRunResult::success());
		expect(&*world).not().to_have_component::<Running>(entity);
	}
}
