use crate::prelude::*;
use bevy::ecs::component::Component;
use bevy::ecs::observer::Trigger;
use bevy::ecs::system::Query;
use bevy::ecs::world::OnRemove;
use bevy::prelude::Commands;
use bevy::prelude::Entity;
use std::marker::PhantomData;


/// Use this builder inside `Component::register_component_hooks` to add observers to an entity.
/// They will be removed when the action component is removed.
pub struct ActionObserversBuilder<T, M, Observers: IntoActionObservers<M>> {
	observers: Observers,
	phantom: PhantomData<(T, M)>,
}

impl<T> Default for ActionObserversBuilder<T, (), ()> {
	fn default() -> Self {
		ActionObserversBuilder {
			observers: (),
			phantom: PhantomData,
		}
	}
}

impl ActionObserversBuilder<(), (), ()> {
	pub fn new<T>() -> ActionObserversBuilder<T, (), ()> { Default::default() }

	/// Enables clean runtime modification of actions.
	/// Removing a behavior entity will automatically remove all its observers (bevy does this).
	///
	/// Beet extends this to the removal of particular action components,
	/// which will remove that action's observers.
	///
	/// This is registered as an OnRemove observer, so that it runs AFTER bevy's built-in
	/// on_remove hook on the internal `ObservedBy` component, which tracks observers watching the
	/// entity and potentially despawns them when the entity despawns.
	///
	/// This is important because the built-in on_remove hook for ObservedBy uses `despawn`, not
	/// `try_despawn`, so upon despawning the entity with the action component, we want the built-in
	/// ObservedBy on_remove hook to run first, and then we follow-up using `try_despawn` here.
	pub fn cleanup_trigger<T: Component>(
		trigger: Trigger<OnRemove, T>,
		q: Query<&ActionObserverMap<T>>,
		mut commands: Commands,
	) {
		if let Ok(observer_map) = q.get(trigger.entity()) {
			for observer in observer_map.observers.iter() {
				// if t.entity() was just despawned (triggering this OnRemove), then this
				// try_despawn will be a no-op, because bevy will have already despawned the
				// observers in the ObservedBy on_remove hook.
				//
				// If OnRemove was triggered because the action component was removed, but the
				// trigger.entity() still lives, then this will correctly remove the observers.
				commands.entity(*observer).try_despawn();
			}
		}
	}
}

impl<T: 'static + Send + Sync, M, O: IntoActionObservers<M> + Clone>
	ActionObserversBuilder<T, M, O>
{
	pub fn add_observers<O2: IntoActionObservers<M2>, M2>(
		self,
		next: O2,
	) -> ActionObserversBuilder<
		T,
		((M, M2), IntoActionObserversTupleMarker),
		(O, O2),
	> {
		ActionObserversBuilder {
			observers: (self.observers, next),
			phantom: PhantomData,
		}
	}

	pub fn build(self, mut commands: Commands, entity: Entity) {
		let entities = self.observers.spawn_observers(&mut commands, entity);
		commands
			.entity(entity)
			.insert(ActionObserverMap::<T>::new(entities));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn default_removal() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<SequenceFlow>::default());
		let world = app.world_mut();

		let entity = world.spawn(SequenceFlow).id();

		// each action component type spawns a global observer (that's the +1)
		expect(world.entities().len()).to_be(1 + 1);
		world.flush();
		expect(world.entities().len()).to_be(3 + 1);

		world.entity_mut(entity).despawn();
		// observers automatically removed
		expect(world.entities().len()).to_be(0 + 1);
	}
	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<SequenceFlow>::default());
		let world = app.world_mut();

		let entity = world.spawn(SequenceFlow).id();

		// each action component type spawns a global observer (that's the +1)
		expect(world.entities().len()).to_be(1 + 1);
		world.flush();
		expect(world.entities().len()).to_be(3 + 1);

		// just removing an action will remove the observers
		world.entity_mut(entity).remove::<SequenceFlow>();
		expect(world.entities().len()).to_be(3 + 1);
		world.flush();
		expect(world.entities().len()).to_be(1 + 1);
	}
}
