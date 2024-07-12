use bevy::ecs::system::EntityCommands;
use bevy::prelude::*;

#[extend::ext]
pub impl<'w> EntityCommands<'w> {
	/// Sends a Trigger for this entity. This will run any Observer of the event that watches this entity.
	fn trigger<E: Event>(&mut self, event: E) -> &mut Self {
		let entity = self.id();
		self.commands().trigger_targets(event, entity);
		self
	}
}
