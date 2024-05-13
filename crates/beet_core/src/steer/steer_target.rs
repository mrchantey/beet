use anyhow::Result;
use bevy::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component)]
// Description of a target used by steering behaviors.
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
	/// Get either the fixed position or the entity's transform, dependent on the variant.
	pub fn position(&self, query: &Query<&Transform>) -> Result<Vec3> {
		match self {
			Self::Position(position) => Ok(*position),
			Self::Entity(entity) => {
				if let Ok(transform) = query.get(*entity) {
					Ok(transform.translation)
				} else {
					anyhow::bail!("transform not found for entity {entity:?}")
				}
			}
		}
	}
}

impl Into<SteerTarget> for Vec3 {
	fn into(self) -> SteerTarget { SteerTarget::Position(self) }
}
impl Into<SteerTarget> for Entity {
	fn into(self) -> SteerTarget { SteerTarget::Entity(self) }
}
