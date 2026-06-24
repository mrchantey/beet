use beet_core::prelude::*;

/// Description of a target used by steering behaviors.
/// This can either be a fixed position or an entity with a [`Transform`] component.
#[derive(Debug, Copy, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, MapEntities, Default)]
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
					bevybail!("transform not found for entity {entity:?}")
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

/// Points an agent's [`SteerTarget`] at another entity addressable by a markup
/// entity-ref, eg `<Ship {SteerTargetEntity{target:$planet}}>` with a sibling
/// `<Planet bx:ref="planet"/>`. A markup ref reaches a reflect *component* field but
/// not an enum variant or a template prop, so [`SteerTarget`] (an enum) cannot take
/// the ref directly; this struct holds the `Entity` in a field, and [`on_insert`]
/// sets the real `SteerTarget::Entity` on add. The `#[entities]` field keeps the ref
/// valid through scene (de)serialization, like [`SteerTarget`] itself.
///
/// [`on_insert`]: insert_steer_target
#[derive(Debug, Clone, Component, Reflect, MapEntities)]
#[reflect(Component, MapEntities, Default)]
pub struct SteerTargetEntity {
	/// The entity to steer toward, resolved from a markup `$ref`.
	#[entities]
	pub target: Entity,
}

impl Default for SteerTargetEntity {
	fn default() -> Self {
		Self {
			target: Entity::PLACEHOLDER,
		}
	}
}

/// On add, set the real [`SteerTarget::Entity`] from the markup-resolved ref.
pub(super) fn insert_steer_target(
	ev: On<Insert, SteerTargetEntity>,
	query: Query<&SteerTargetEntity>,
	mut commands: Commands,
) {
	if let Ok(target) = query.get(ev.entity) {
		commands
			.entity(ev.entity)
			.insert(SteerTarget::Entity(target.target));
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	// `SteerTargetEntity` (the markup entity-ref keystone) sets the real
	// `SteerTarget::Entity` on add, so a `<Ship {SteerTargetEntity{target:$planet}}>`
	// steers toward the referenced entity.
	#[beet_core::test]
	fn steer_target_entity_resolves() {
		let mut app = App::new();
		app.add_plugins(BeetSpatialPlugins).init_resource::<Time>();
		let target = app.world_mut().spawn_empty().id();
		let agent = app.world_mut().spawn(SteerTargetEntity { target }).id();
		app.update();
		app.world()
			.get::<SteerTarget>(agent)
			.copied()
			.unwrap()
			.xpect_eq(SteerTarget::Entity(target));
	}
}
