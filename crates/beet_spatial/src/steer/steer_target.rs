use anyhow::Result;
use beet_flow::prelude::*;
use bevy::ecs::entity::MapEntities;
use bevy::ecs::reflect::ReflectMapEntities;
use beet_core::prelude::*;


/// Description of a target used by steering behaviors.
/// This can either be a fixed position or an entity with a [`Transform`] component.
#[derive(Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub enum SteerTarget {
	/// The target is a fixed position
	Position(Vec3),
	/// The target is an entity with a [`Transform`] component
	Entity(Entity),
}
impl Default for SteerTarget {
	fn default() -> Self { Self::Position(Vec3::default()) }
}

impl SteerTarget {
	/// Get either the fixed position or the entity's `Transform.translation`, dependent on the variant.
	/// # Errors
	/// If the variant is `SteerTarget::Entity` and no `Transform` could be found.
	pub fn get_position(
		&self,
		query: &Query<&GlobalTransform>,
	) -> Result<Vec3> {
		match self {
			Self::Position(position) => Ok(*position),
			Self::Entity(entity) => {
				if let Ok(transform) = query.get(*entity) {
					Ok(transform.translation())
				} else {
					anyhow::bail!("transform not found for entity {entity:?}")
				}
			}
		}
	}
}


impl MapEntities for SteerTarget {
	fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
		match self {
			Self::Entity(entity) => *entity = entity_mapper.get_mapped(*entity),
			_ => {}
		}
	}
}


impl Into<SteerTarget> for Vec3 {
	fn into(self) -> SteerTarget { SteerTarget::Position(self) }
}
impl Into<SteerTarget> for Entity {
	fn into(self) -> SteerTarget { SteerTarget::Entity(self) }
}
