use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Do something whenever `OnChange` is called on a component
#[derive(Component, Action, Reflect)]
#[reflect(Default, Component, MapEntities)]
#[systems(on_change::<Handler>.in_set(TickSet))]
pub struct OnChange<Handler: OnChangeHandler> {
	pub params: Handler::Params,
	/// Source to watch, if Global then all entities are watched
	pub source: ActionTarget,
	pub target: ActionTarget,
	#[reflect(ignore)]
	phantom: PhantomData<Handler>,
}

impl<Handler: OnChangeHandler> Default for OnChange<Handler>
where
	Handler::Params: Default,
{
	fn default() -> Self {
		Self {
			params: default(),
			source: default(),
			target: default(),
			phantom: PhantomData,
		}
	}
}

impl<Handler: OnChangeHandler> MapEntities for OnChange<Handler> {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.source.map_entities(entity_mapper);
		self.target.map_entities(entity_mapper);
	}
}

impl<Handler: OnChangeHandler> OnChange<Handler> {
	pub fn new() -> Self { Self::default() }
	pub fn new_with_source(target: impl Into<ActionTarget>) -> Self {
		Self {
			source: target.into(),
			..default()
		}
	}
	pub fn new_with_target(target: impl Into<ActionTarget>) -> Self {
		Self {
			target: target.into(),
			..default()
		}
	}

	pub fn with_source(self, target: impl Into<ActionTarget>) -> Self {
		Self {
			source: target.into(),
			..self
		}
	}
	pub fn with_target(self, target: impl Into<ActionTarget>) -> Self {
		Self {
			target: target.into(),
			..self
		}
	}
}

fn on_change<Handler: OnChangeHandler>(
	actions: Query<(Entity, &OnChange<Handler>)>,
	query: Query<
		(Entity, &Handler::ChangedComponent),
		Changed<Handler::ChangedComponent>,
	>,
	mut commands: Commands,
) {
	for (changed_entity, changed_component) in query.iter() {
		for (action_entity, action) in actions.iter() {
			match &action.source {
				ActionTarget::This => {
					if changed_entity != action_entity {
						continue;
					}
				}
				ActionTarget::Entity(source) => {
					if source != &action_entity {
						continue;
					}
				}
				ActionTarget::Entities(entities) => {
					if !entities.contains(&action_entity) {
						continue;
					}
				}
				ActionTarget::Global => {
					// all entities are watched
				}
			}
			Handler::handle(
				&mut commands,
				action_entity,
				&action,
				changed_entity,
				changed_component,
			);
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> { Ok(()) }
	#[test]
	fn other_sources() -> Result<()> { Ok(()) }
}
