use crate::prelude::*;
use bevy_ecs::prelude::*;

pub trait WorldOrCommands {
	fn spawn(&mut self, bundle: impl Bundle) -> Entity;
	fn insert(&mut self, entity: Entity, bundle: impl Bundle);
	fn apply_action(&mut self, action: &dyn Action, entity: Entity);
	fn apply_action_typed<T: Action>(&mut self, action: &T, entity: Entity);
}

impl WorldOrCommands for World {
	fn spawn(&mut self, bundle: impl Bundle) -> Entity {
		self.spawn(bundle).id()
	}
	fn insert(&mut self, entity: Entity, bundle: impl Bundle) {
		self.entity_mut(entity).insert(bundle);
	}
	fn apply_action(&mut self, action: &dyn Action, entity: Entity) {
		action.spawn(&mut self.entity_mut(entity));
	}
	fn apply_action_typed<T: Action>(&mut self, action: &T, entity: Entity) {
		action.spawn(&mut self.entity_mut(entity));
	}
}
impl<'w, 's> WorldOrCommands for Commands<'w, 's> {
	fn spawn(&mut self, bundle: impl Bundle) -> Entity {
		self.spawn(bundle).id()
	}
	fn insert(&mut self, entity: Entity, bundle: impl Bundle) {
		self.entity(entity).insert(bundle);
	}
	fn apply_action(&mut self, action: &dyn Action, entity: Entity) {
		action.spawn_with_command(&mut self.entity(entity));
	}
	fn apply_action_typed<T: Action>(&mut self, action: &T, entity: Entity) {
		action.spawn_with_command(&mut self.entity(entity));
	}
}
