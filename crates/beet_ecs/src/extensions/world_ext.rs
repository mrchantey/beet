use bevy::ecs::system::IntoObserverSystem;
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
