use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::HashMap;

/// An Action Entity is any node on a control flow graph,
/// containing the action components.
#[allow(unused, reason = "docs only")]
pub(crate) type ActionEntity = ActionObservers;


/// An Action Observer Entity is a single entity created
/// for each action definition, forming a many-to-many
/// relationship with each [ActionEntity] that holds that
/// action. This structure ensures that only the observers
/// that are needed are run.
#[derive(Debug, Component)]
pub struct ActionObserver;


/// A component added to any entity with an action, it tracks
/// the observers that are listening to the action.
/// This will likely become a many-many relationship when bevy supports it.
#[derive(Debug, Default, Component, Deref, DerefMut)]
pub struct ActionObservers(pub Vec<Entity>);

/// Tracks action observers created when the first instance of an action is
/// created.
#[derive(Debug, Default, Resource, Deref, DerefMut)]
pub struct ActionObserverMap(pub HashMap<ComponentId, Entity>);

impl ActionObserverMap {
	fn get_or_spawn(
		world: &mut DeferredWorld,
		cid: ComponentId,
		mut on_spawn: impl FnMut(&mut DeferredWorld, Entity),
	) -> Entity {
		let map = world.resource::<Self>();
		if let Some(action) = map.get(&cid) {
			return *action;
		}
		let observer_entity = world.commands().spawn(ActionObserver).id();
		on_spawn(world, observer_entity);
		let mut map = world.resource_mut::<Self>();
		map.insert(cid, observer_entity);
		observer_entity
	}
}

impl ActionObservers {
	/// Called whenever an action is added to an [`ActionEntity`].
	/// Do not call this directly, it is called by the `#[action]` macro component hooks.
	pub fn on_add(
		world: &mut DeferredWorld,
		action: Entity,
		cid: ComponentId,
		on_spawn_observer: impl FnMut(&mut DeferredWorld, Entity),
	) {
		let observer_entity =
			ActionObserverMap::get_or_spawn(world, cid, on_spawn_observer);


		world
			.commands()
			.entity(action)
			.entry::<ActionObservers>()
			// should always exist because macro adds
			// #[require(ActionObservers)]
			.or_default()
			.and_modify(move |mut actions| actions.push(observer_entity));
	}
	/// Called whenever an action is removed from an [`ActionEntity`].
	/// Do not call this directly, it is called by the `#[action]` macro component hooks.
	pub fn on_remove(world: &mut DeferredWorld, action: Entity) {
		if let Some(mut actions) = world.get_mut::<ActionObservers>(action) {
			actions.retain(|&e| e != action);
		}
	}
}

pub fn on_remove_action(
	mut world: DeferredWorld,
	action: Entity,
	_cid: ComponentId,
) {
	ActionObservers::on_remove(&mut world, action);
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn multiple_actions() {
		let mut app = App::new();
		app.add_plugins(BeetFlowPlugin::default());
		let world = app.world_mut();

		let entity = world
			.spawn((
				ReturnWith(ScoreValue::NEUTRAL),
				ReturnWith(RunResult::Success),
			))
			.id();

		world.flush();
		let observers = world.get::<ActionObservers>(entity).unwrap();
		expect(observers.len()).to_be(2);
	}
}
