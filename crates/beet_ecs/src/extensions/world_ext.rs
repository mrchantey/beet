use bevy::prelude::*;



#[extend::ext]
pub impl<'w> EntityWorldMut<'w> {
	/// Triggers the given event for this entity, which will run any observers watching for it.
	fn trigger<E: Event>(&mut self, event: E) -> &mut Self {
		let entity = self.id();
		unsafe {
			self.world_mut().trigger_targets(event, entity);
		}
		self
	}
}
