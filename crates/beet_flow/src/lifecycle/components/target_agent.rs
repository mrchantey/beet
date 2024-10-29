use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;

/// This component will be replaced with a [`TargetAgent`] that points to the root [`Parent`] of this entity.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RootIsTargetAgent;

/// Attach this to behavior entities that require a target agent.
#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct TargetAgent(pub Entity);

impl MapEntities for TargetAgent {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}

pub fn set_root_as_target_agent(
	mut commands: Commands,
	parents: Query<&Parent>,
	query: Query<Entity, With<RootIsTargetAgent>>,
) {
	for entity in query.iter() {
		let root = parents.root_ancestor(entity);
		commands
			.entity(entity)
			.remove::<RootIsTargetAgent>()
			.insert(TargetAgent(root));
	}
}
