use crate::prelude::*;
use bevy::ecs::lifecycle::HookContext;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;


/// This will add the [`Running`] component to the behavior when [`OnRun`] is triggered,
/// and remove it when [`OnResult`] is triggered.
///
/// This should be added as `#[require(ContinueRun)]` for any long running action,
/// ie any action that has a [`With<Running>`] query filter.
/// It should not added to behaviors directly, because its easy to forget.
/// For usage see the [`Running`] component.
#[action(insert_running)]
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RunTimer,Remove<OnResult,Running>)]
pub struct ContinueRun;

fn insert_running(ev: On<OnRun>, mut commands: Commands) {
	commands.entity(ev.action).insert(Running::new(ev.origin));
}

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
#[derive(Debug, Copy, Clone, Component, PartialEq, Reflect)]
#[component(storage = "SparseSet",on_add = on_add_running)]
#[reflect(Component)]
#[require(RunTimer)] // mostly for tests where we added running directly, usually this is required by `ContinueRun`
pub struct Running {
	/// The entity upon which actions can perform some work, often the
	/// root of the action tree but can be any entity.
	pub origin: Entity,
}

/// if Running was added with a placeholder origin, set it to the entity it was added to.
fn on_add_running(mut world: DeferredWorld, cx: HookContext) {
	let mut running = world.get_mut::<Running>(cx.entity).unwrap();
	if running.origin == Entity::PLACEHOLDER {
		running.origin = cx.entity;
	}
}

impl Running {
	/// Create a new instance of `Running` with the provided origin.
	pub fn new(origin: Entity) -> Self { Self { origin } }


	/// Trigger a result, the action must be the entity containing this [`Running`] component.
	pub fn trigger_result<T: ResultPayload>(
		&self,
		commands: &mut Commands,
		action: Entity,
		payload: T,
	) {
		commands.trigger(OnResultAction::new(action, self.origin, payload));
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
		world.get::<Running>(entity).xpect_some();
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
		world.get::<Running>(entity).xpect_none();
	}

	#[test]
	fn sets_orgin_on_add_default() {
		let mut world = World::new();
		let entity = world.spawn(Running::default()).id();
		world
			.get::<Running>(entity)
			.unwrap()
			.xpect_eq(Running { origin: entity });
	}

	#[test]
	fn sets_origin_on_continue_run() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();
		let origin = world.spawn_empty().id();
		let action = world.spawn(ContinueRun).id();
		world.flush_trigger(OnRunAction::new(action, origin, ()));

		world
			.get::<Running>(action)
			.unwrap()
			.xpect_eq(Running::new(origin));
	}
}
