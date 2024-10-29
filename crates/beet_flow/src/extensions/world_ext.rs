use bevy::ecs::system::IntoObserverSystem;
use bevy::ecs::system::SystemState;
use bevy::prelude::*;



#[extend::ext]
pub impl World {
	fn with_observer<E: Event, B: Bundle, M>(
		mut self,
		system: impl IntoObserverSystem<E, B, M>,
	) -> Self {
		self.spawn(Observer::new(system));
		self
	}
	fn observing<E: Event, B: Bundle, M>(
		&mut self,
		system: impl IntoObserverSystem<E, B, M>,
	) -> &mut Self {
		self.spawn(Observer::new(system));
		self
	}

	fn flush_trigger<E: Event>(&mut self, event: E) -> &mut Self {
		self.flush();
		self.trigger(event);
		self.flush();
		self
	}
	fn collect_descendants(&mut self, entity: Entity) -> Vec<Entity> {
		let mut state = SystemState::<Query<&Children>>::new(self);
		state.get(&self).iter_descendants(entity).collect()
	}
	fn collect_descendants_inclusive(&mut self, entity: Entity) -> Vec<Entity> {
		let mut state = SystemState::<Query<&Children>>::new(self);
		let state = state.get(&self);
		std::iter::once(entity)
			.chain(state.iter_descendants(entity))
			.collect()
	}
}

#[extend::ext]
pub impl<'w> EntityWorldMut<'w> {
	/// 1. Flushes
	/// 2. Triggers the given event for this entity, which will run any observers watching for it.
	/// 3. Flushes
	fn flush_trigger<E: Event>(&mut self, event: E) -> &mut Self {
		let entity = self.id();
		unsafe {
			let world = self.world_mut();
			world.flush();
			world.trigger_targets(event, entity);
			world.flush();
		}
		self
	}
}
