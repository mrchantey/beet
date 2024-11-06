use crate::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

// Represents a relationship between variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalRelationship {
	source: Entity,
	target: Entity,
	function: CausalFunction,
}


pub enum RelationshipSource {
	Entity(Entity),
	Constant(EmbyFactorId),
}

impl MapEntities for RelationshipSource {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		match self {
			RelationshipSource::Entity(entity) => {
				*entity = entity_mapper.map_entity(*entity);
			}
			RelationshipSource::Constant(_) => {}
		}
	}
}


impl MapEntities for CausalRelationship {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		self.source = entity_mapper.map_entity(self.source);
		self.target = entity_mapper.map_entity(self.target);
	}
}
