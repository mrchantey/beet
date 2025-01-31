use crate::prelude::*;
use bevy::ecs::world::DeferredWorld;
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
	/// Removing a behavior entity will automatically remove all its observers.
	/// In addition to this Beet extends this to the removal of particular action components,
	/// which will remove that action's observers.
	///
	/// this function is called for the removal of an action component
	/// as well as the removal of the entity itself, if the entity is removed
	/// bevy will handle the clean up for us so `try_despawn` is used.
	pub fn cleanup<'w, T: 'static + Send + Sync>(
		world: &mut DeferredWorld<'w>,
		entity: Entity,
	) {
		if let Some(observers) = world
			.entity(entity)
			.get::<ActionObserverMap<T>>()
			.map(|obs| (*obs).clone())
		{
			let mut commands = world.commands();
			for observer in observers.observers.iter() {
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
	use ::sweet::prelude::*;

	#[test]
	fn default_removal() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<SequenceFlow>::default());
		let world = app.world_mut();

		let entity = world.spawn(SequenceFlow).id();

		expect(world.entities().len()).to_be(1);
		world.flush();
		expect(world.entities().len()).to_be(3);

		world.entity_mut(entity).despawn();
		// observers automatically removed
		expect(world.entities().len()).to_be(0);
	}
	#[test]
	fn works() {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<SequenceFlow>::default());
		let world = app.world_mut();

		let entity = world.spawn(SequenceFlow).id();

		expect(world.entities().len()).to_be(1);
		world.flush();
		expect(world.entities().len()).to_be(3);

		// just removing an action will remove the observers
		world.entity_mut(entity).remove::<SequenceFlow>();
		expect(world.entities().len()).to_be(3);
		world.flush();
		expect(world.entities().len()).to_be(1);
	}
}
