use crate::prelude::*;
use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;


/// This will add the [`Running`] component to the behavior when [`OnRun`] is triggered,
/// and remove it when [`OnResult`] is triggered.
///
/// This should be added as `#[require(ContinueRun)]` for any long running action,
/// ie any action that has a [`With<Running>`] query filter.
/// It should not added to behaviors directly, because its easy to forget.
/// For usage see the [`Running`] component.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer,Insert<OnRun,Running>,Remove<OnResult,Running>)]
pub struct ContinueRun;


/// A marker component added to an [ActionEntity] indicate this action is currently running.
/// ```
///	# use bevy::prelude::*;
///	# use beet_flow::prelude::*;
///
/// #[derive(Component)]
/// #[require(ContinueRun)]
/// struct MyLongAction;
///
/// fn my_long_action(query: Query<&MyLongAction, With<Running>>){
/// 	for action in query.iter(){
/// 	  // etc.
/// 	}
/// }
/// ```
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet",on_add = on_add_running)]
#[reflect(Component)]
pub struct Running {
	/// The entity upon which actions can perform some work, often the
	/// root of the action tree but can be any entity.
	pub origin: Entity,
}
impl Running {
	/// Create a new instance of `Running` with the provided origin.
	pub fn new(origin: Entity) -> Self { Self { origin } }
}

fn on_add_running(mut world: DeferredWorld, entity: Entity, _cid: ComponentId) {
	let mut running = world.get_mut::<Running>(entity).unwrap();
	if running.origin == Entity::PLACEHOLDER {
		running.origin = entity;
	}
}
/// Like [`OnRun::local`], this will resolve to the entity it was placed on
/// in the `on_add` component hook.
impl Default for Running {
	fn default() -> Self {
		Self {
			origin: Entity::PLACEHOLDER,
		}
	}
}


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
		let entity = world
			.spawn((Running::default(), ContinueRun))
			.flush_trigger(OnResultAction::local(RunResult::Success))
			.id();
		expect(world.get::<Running>(entity)).to_be_none();
	}
}
