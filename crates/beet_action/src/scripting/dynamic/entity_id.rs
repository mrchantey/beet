//! The one place the entity wire form lives.
//!
//! An entity crosses to a script as an opaque string in bevy's own display form
//! (`"42v3"`), never as a number: [`Entity::to_bits`] is a `u64` and a JS number
//! loses precision past 2^53, so a round trip through JSON would silently hand
//! back a different entity. A script treats the string as a token to pass back,
//! which is exactly what it is.
use beet_core::prelude::*;
use bevy::ecs::entity::EntityGeneration;
use bevy::ecs::entity::EntityIndex;

/// `entity` as the string a script holds it by.
pub fn encode(entity: Entity) -> String { entity.to_string() }

/// The entity a script's `"42v3"` names.
///
/// # Errors
/// Errors naming the token when it is not in the display form.
pub fn decode(id: &str) -> Result<Entity> {
	let malformed =
		|| bevyhow!("`{id}` is not an entity id, which reads like `42v1`");
	let (index, generation) = id.split_once('v').ok_or_else(malformed)?;
	let index = index
		.parse()
		.ok()
		.and_then(EntityIndex::from_raw_u32)
		.ok_or_else(malformed)?;
	let generation = generation
		.parse()
		.map(EntityGeneration::from_bits)
		.map_err(|_| malformed())?;
	Entity::from_index_and_generation(index, generation).xok()
}

#[cfg(test)]
mod test {
	use crate::scripting::dynamic::entity_id;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn round_trips_a_spawned_entity() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		entity_id::decode(&entity_id::encode(entity))
			.unwrap()
			.xpect_eq(entity);
	}

	/// The generation is part of the token, so a reused index does not silently
	/// resolve to the entity that replaced it.
	#[beet_core::test]
	fn round_trips_a_reused_index() {
		let mut world = World::new();
		let first = world.spawn_empty().id();
		world.entity_mut(first).despawn();
		let second = world.spawn_empty().id();
		second.xpect_not_eq(first);
		entity_id::decode(&entity_id::encode(second))
			.unwrap()
			.xpect_eq(second);
	}

	#[beet_core::test]
	fn a_malformed_id_names_itself() {
		entity_id::decode("nonesuch")
			.unwrap_err()
			.to_string()
			.xpect_contains("`nonesuch` is not an entity id");
	}
}
