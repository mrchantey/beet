//! Components required for long running actions.
use crate::prelude::*;
use beet_core::prelude::*;


/// This will add the [`Running`] component to the behavior when [`OnRun`] is triggered,
/// and remove it when [`OnResult`] is triggered.
///
/// This should be added as `#[require(ContinueRun)]` for any long running action,
/// ie any action that has a [`With<Running>`] query filter.
/// It should not added to behaviors directly, because its easy to forget.
/// For usage see the [`Running`] component.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer, InsertOn<Run, Running>, RemoveOn<End, Running>)]
pub struct ContinueRun;

/// A marker component added to an [ActionEntity] indicate this action is currently running.
/// ## Example
/// This is the `Translate` action found in `beet_spatial`.
/// ```
///	# use bevy::prelude::*;
///	# use beet_flow::prelude::*;
///
/// #[derive(Component)]
/// #[require(ContinueRun)]
/// struct Translate(pub Vec3);
///
/// fn translate(
/// 	time: Res<Time>,
/// 	action: Query<(&Running, &Translate)>,
/// 	mut transforms: Query<&mut Transform>,
/// ){
/// 	for (running, translate) in action.iter(){
/// 		let mut transform = transforms
/// 			.get_mut(running.origin)
/// 			.expect(&expect_action::to_have_origin(&running));
/// 		transform.translation += translate.0 * time.delta_secs();
/// 	}
/// }
/// ```
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Debug, Default, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet")]
#[reflect(Component)]
#[require(RunTimer)]
pub struct Running;


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn adds() {
		let mut app = App::new();
		// app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		// adds
		let entity = world.spawn(ContinueRun).id();
		world.get::<Running>(entity).xpect_none();
		world.entity_mut(entity).trigger_target(RUN);
		world.get::<Running>(entity).xpect_some();
		world.entity_mut(entity).trigger_target(SUCCESS);
		world.get::<Running>(entity).xpect_none();
	}
}
