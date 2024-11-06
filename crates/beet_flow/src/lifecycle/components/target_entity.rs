use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use bevy::prelude::*;

/// Very simple pre-entity relations mechanic, 
/// add this as an outgoing relation to entities with actions and other components that require it.
#[derive(Debug, PartialEq, Deref, DerefMut, Component, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct TargetEntity(pub Entity);

impl MapEntities for TargetEntity {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		**self = entity_mapper.map_entity(**self);
	}
}

/// This component will be replaced with a [`TargetEntity`] that points to the root [`Parent`] of this entity.
#[derive(Debug, Default, Component, Reflect)]
#[reflect(Component, Default)]
pub struct RootIsTargetEntity;

pub fn set_root_as_target_entity(
	mut commands: Commands,
	parents: Query<&Parent>,
	query: Query<Entity, With<RootIsTargetEntity>>,
) {
	for entity in query.iter() {
		let root = parents.root_ancestor(entity);
		commands
			.entity(entity)
			.remove::<RootIsTargetEntity>()
			.insert(TargetEntity(root));
	}
}
