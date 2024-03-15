use anyhow::Result;
use bevy::prelude::*;

#[derive(Debug, Copy, Clone, PartialEq, Component)]
pub enum SteerTarget {
	Position(Vec3),
	Entity(Entity),
}
impl Default for SteerTarget {
	fn default() -> Self { Self::Position(Vec3::default()) }
}

impl SteerTarget {
	// TODO 0.13 query lens
	pub fn position(&self, query: &Query<&Transform>) -> Result<Vec3> {
		match self {
			Self::Position(position) => Ok(*position),
			Self::Entity(entity) => {
				if let Ok(transform) = query.get(*entity) {
					Ok(transform.translation)
				} else {
					anyhow::bail!("entity {entity:?} not found")
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
