use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;
use std::marker::PhantomData;


/// Run whenever `OnChange` is called on a component
/// This may be deprecated if we get `OnChange` observers
#[derive(Component, Action, Reflect)]
#[reflect(Default, Component, MapEntities)]
#[systems(on_change::<C>.in_set(TickSet))]
pub struct RunOnChange<C: Component> {
	/// Source to watch, if Global then all entities are watched
	pub source: ActionTarget,
	#[reflect(ignore)]
	phantom: PhantomData<C>,
}

impl<C: Component> MapEntities for RunOnChange<C> {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.source.map_entities(entity_mapper);
	}
}

impl<C: Component> Default for RunOnChange<C> {
	fn default() -> Self {
		Self {
			source: default(),
			phantom: default(),
		}
	}
}

impl<C: Component> RunOnChange<C> {
	pub fn new() -> Self { Self::default() }
	pub fn new_with_source(target: impl Into<ActionTarget>) -> Self {
		Self {
			source: target.into(),
			..default()
		}
	}	
	pub fn with_source(self, target: impl Into<ActionTarget>) -> Self {
		Self {
			source: target.into(),
			..self
		}
	}
}

fn on_change<C: Component>(
	actions: Query<(Entity, &RunOnChange<C>)>,
	query: Query<Entity, Changed<C>>,
	mut commands: Commands,
) {
	for changed_entity in query.iter() {
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
					// never continue, all entities are watched
				}
			}
			commands.entity(action_entity).trigger(OnRun);
		}
	}
}

#[cfg(test)]
mod test {
	use super::RunOnChange;
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(ActionPlugin::<RunOnChange<Running>>::default());

		let world = app.world_mut();
		let entity = world
			.spawn((Running, RunOnChange::<Running>::default()))
			.id();

		app.update();

		expect(app.world_mut().get::<Running>(entity)).to_be_some()?;


		Ok(())
	}
}
