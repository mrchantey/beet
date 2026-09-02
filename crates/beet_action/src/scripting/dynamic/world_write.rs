//! The one place a script's mutations reach the world.
//!
//! The mirror of [`WorldRead`]: four operations, each checked against the
//! exposure's write filter before it touches anything, each landing the moment
//! the script asks for it. Every one tolerates a target that has gone: a
//! despawned entity is an error naming it, never a panic.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ptr::OwningPtr;
use bevy::reflect::serde::TypedReflectDeserializer;
use serde::de::DeserializeSeed;
use serde_json::Map as JsonMap;
use serde_json::Value as JsonValue;

/// The four world mutations a script can express.
pub struct WorldWrite;

impl WorldWrite {
	/// Insert or replace `ident` on `entity`.
	///
	/// # Errors
	/// Errors when the identifier is unknown, the exposure excludes it, the
	/// value does not deserialize into the component, or the entity is gone.
	pub fn insert(
		world: &mut World,
		entity: Entity,
		ident: &str,
		value: &JsonValue,
		exposure: &ScriptExposure,
	) -> Result {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_writable(&ident)?;
		match ident.type_id {
			None => {
				let value = Value::from_json(value.clone());
				Self::entity_mut(world, entity)?.xmap(|mut entity| {
					// SAFETY: a dynamic `ComponentId` only ever comes from
					// `DynamicComponents::register`, which declares a `Value` layout
					// and a matching drop fn, and `value` is a `Value` given away
					// here rather than dropped.
					unsafe {
						OwningPtr::make(value, |ptr| {
							entity.insert_by_id(ident.id, ptr);
						})
					};
				});
				Ok(())
			}
			Some(type_id) => {
				let registry = world.resource::<AppTypeRegistry>().clone();
				let value = {
					let registry = registry.read();
					let registration =
						registry.get(type_id).ok_or_else(|| {
							bevyhow!("`{}` left the registry", ident.path)
						})?;
					TypedReflectDeserializer::new(registration, &registry)
						.deserialize(value)
						.map_err(|err| {
							bevyhow!(
								"failed to read `{}` from {value}: {err}",
								ident.path
							)
						})?
				};
				let registry = registry.read();
				let reflect_component = registry
					.get(type_id)
					.and_then(|registration| {
						registration.data::<ReflectComponent>()
					})
					.ok_or_else(|| {
						bevyhow!("`{}` is not a component", ident.path)
					})?
					.clone();
				let mut entity = Self::entity_mut(world, entity)?;
				reflect_component.insert(&mut entity, &*value, &registry);
				Ok(())
			}
		}
	}

	/// Remove `ident` from `entity`, tolerating an entity that never had it.
	///
	/// # Errors
	/// Errors when the identifier is unknown, the exposure excludes it, or the
	/// entity is gone.
	pub fn remove(
		world: &mut World,
		entity: Entity,
		ident: &str,
		exposure: &ScriptExposure,
	) -> Result {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_writable(&ident)?;
		Self::entity_mut(world, entity)?.remove_by_id(ident.id);
		Ok(())
	}

	/// Spawn an entity carrying `components`.
	///
	/// # Errors
	/// Errors on the first component that fails to insert; the entity is
	/// despawned again rather than left half built.
	pub fn spawn(
		world: &mut World,
		components: &JsonMap<String, JsonValue>,
		exposure: &ScriptExposure,
	) -> Result<Entity> {
		let entity = world.spawn_empty().id();
		for (ident, value) in components {
			if let Err(err) =
				Self::insert(world, entity, ident, value, exposure)
			{
				world.entity_mut(entity).despawn();
				return Err(err);
			}
		}
		entity.xok()
	}

	/// Despawn `entity`.
	///
	/// # Errors
	/// Errors naming the entity when it is already gone.
	pub fn despawn(world: &mut World, entity: Entity) -> Result {
		Self::entity_mut(world, entity)?.despawn();
		Ok(())
	}

	/// The entity, as an error rather than a panic when the world despawned it
	/// between the script obtaining its id and asking for this write.
	fn entity_mut<'w>(
		world: &'w mut World,
		entity: Entity,
	) -> Result<EntityWorldMut<'w>> {
		world.get_entity_mut(entity).map_err(|_| {
			bevyhow!("{entity} was despawned before the script's write landed")
		})
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn inserts_a_registered_component() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"Name",
			&serde_json::json!("ada"),
			&ScriptExposure::default(),
		)
		.unwrap();
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.xpect_eq(Name::new("ada"));
	}

	#[beet_core::test]
	fn removes_a_component() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		WorldWrite::remove(
			&mut world,
			entity,
			"Name",
			&ScriptExposure::default(),
		)
		.unwrap();
		world.entity(entity).contains::<Name>().xpect_false();
	}

	#[beet_core::test]
	fn spawns_with_components() {
		let mut world = test_world();
		let entity = WorldWrite::spawn(
			&mut world,
			serde_json::json!({ "Name": "ada" }).as_object().unwrap(),
			&ScriptExposure::default(),
		)
		.unwrap();
		world
			.entity(entity)
			.get::<Name>()
			.unwrap()
			.xpect_eq(Name::new("ada"));
	}

	#[beet_core::test]
	fn despawns() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		WorldWrite::despawn(&mut world, entity).unwrap();
		world.get_entity(entity).is_err().xpect_true();
	}

	/// A half-built spawn is despawned rather than left behind.
	#[beet_core::test]
	fn a_failed_spawn_leaves_nothing() {
		let mut world = test_world();
		WorldWrite::spawn(
			&mut world,
			serde_json::json!({ "Nonesuch": 1 }).as_object().unwrap(),
			&ScriptExposure::default(),
		)
		.unwrap_err();
		entity_count(&world).xpect_eq(0);
	}

	#[beet_core::test]
	fn a_write_outside_the_exposure_names_the_path() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"Name",
			&serde_json::json!("ada"),
			&ScriptExposure::new(["game.Health"]),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("may not write `bevy_ecs::name::Name`");
	}

	/// A target the world despawned between the script taking its id and the
	/// write landing reports, rather than panicking under the default error
	/// handler.
	#[beet_core::test]
	fn a_despawned_target_reports() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		world.entity_mut(entity).despawn();
		WorldWrite::insert(
			&mut world,
			entity,
			"Name",
			&serde_json::json!("bob"),
			&ScriptExposure::default(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("was despawned");
	}
}
