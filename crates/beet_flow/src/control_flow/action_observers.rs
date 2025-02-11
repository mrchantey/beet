use bevy::ecs::component::ComponentId;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::*;
use bevy::utils::HashMap;

/// Marker applied to all action observers used to halt recursion
/// for global observers amongst other things.
#[derive(Debug, Component)]
pub struct ActionObserverMarker;


/// This is added to any entity with an action, it tracks
/// the observers that are listening to the action.
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
		let observer_entity = world.commands().spawn(ActionObserverMarker).id();
		on_spawn(world, observer_entity);
		let mut map = world.resource_mut::<Self>();
		map.insert(cid, observer_entity);
		observer_entity
	}
}

impl ActionObservers {
	// called by the Action component hooks.
	pub fn on_add(
		world: &mut DeferredWorld,
		action: Entity,
		cid: ComponentId,
		on_spawn_observer: impl FnMut(&mut DeferredWorld, Entity),
	) {
		let observer_entity =
			ActionObserverMap::get_or_spawn(world, cid, on_spawn_observer);

		if let Some(mut action_observers) =
			world.get_mut::<ActionObservers>(action)
		{
			action_observers.0.push(observer_entity);
		} else {
			world
				.commands()
				.entity(action)
				.entry::<ActionObservers>()
				.or_default()
				.and_modify(move |mut actions| actions.push(observer_entity));
		}
	}
	// called by the Action component hooks.
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
