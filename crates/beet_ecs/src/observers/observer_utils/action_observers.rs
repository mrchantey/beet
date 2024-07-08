use crate::prelude::*;
use bevy::ecs::world::DeferredWorld;
use bevy::prelude::Commands;
use bevy::prelude::Entity;
use std::marker::PhantomData;


/// Use this builder inside `Component::register_component_hooks` to add observers to an entity.
/// They will be removed when the component is removed.
pub struct ActionObservers<T, M, Systems: ObserverLifecycle<M>> {
	systems: Systems,
	phantom: PhantomData<(T, M)>,
}

impl<T> Default for ActionObservers<T, (), ()> {
	fn default() -> Self {
		ActionObservers {
			systems: (),
			phantom: PhantomData,
		}
	}
}

impl ActionObservers<(), (), ()> {
	pub fn new<T>() -> ActionObservers<T, (), ()> { Default::default() }
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
				commands.entity(*observer).despawn();
			}
		}
	}
}

impl<T: 'static + Send + Sync, M, O: ObserverLifecycle<M> + Clone>
	ActionObservers<T, M, O>
{
	pub fn add_observers<O2: ObserverLifecycle<M2>, M2>(
		self,
		next: O2,
	) -> ActionObservers<T, ((M, M2), ObserverLifecycleTupleMarker), (O, O2)>
	{
		ActionObservers {
			systems: (self.systems, next),
			phantom: PhantomData,
		}
	}
	
	pub fn build(self, mut commands: Commands, entity: Entity) {
		let entities = self.systems.spawn_observers(&mut commands, entity);
		commands
			.entity(entity)
			.insert(ActionObserverMap::<T>::new(entities));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut world = World::new();

		let entity = world.spawn(SequenceFlow).id();

		expect(world.entities().len()).to_be(1)?;
		world.flush();
		expect(world.entities().len()).to_be(3)?;

		world.entity_mut(entity).remove::<SequenceFlow>();
		// world.entity_mut(entity).despawn();
		expect(world.entities().len()).to_be(3)?;
		world.flush();
		expect(world.entities().len()).to_be(1)?;

		Ok(())
	}
}
