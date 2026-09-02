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

/// The four world mutations a script can express, and the schema lookup that
/// precedes one.
pub struct WorldWrite;

impl WorldWrite {
	/// Insert or replace `ident` on `entity`.
	///
	/// A runtime component takes the [`Value`] as it stands, so anything the
	/// wire carries reaches its storage unaltered; a registered one is
	/// deserialized into its rust type, which is its own contract. Whatever a
	/// runtime component's [`DynamicComponent`] declared is checked *before*
	/// this, in [`WorldOp`], where the check can be asynchronous.
	///
	/// # Errors
	/// Errors when the identifier is unknown, the exposure excludes it, the
	/// value does not deserialize into the component, or the entity is gone.
	pub fn insert(
		world: &mut World,
		entity: Entity,
		ident: &str,
		value: Value,
		exposure: &ScriptExposure,
	) -> Result {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_writable(&ident)?;
		match ident.type_id {
			None => {
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
				let reflected = {
					let registry = registry.read();
					let registration =
						registry.get(type_id).ok_or_else(|| {
							bevyhow!("`{}` left the registry", ident.path)
						})?;
					// straight off the `Value`, no JSON hop: `ValueDeserializer`
					// is a real serde data format, so the reflect boundary reads
					// the same currency the rest of the bridge speaks.
					TypedReflectDeserializer::new(registration, &registry)
						.deserialize(ValueDeserializer::new(value.clone()))
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
				reflect_component.insert(&mut entity, &*reflected, &registry);
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
		components: Map,
		exposure: &ScriptExposure,
	) -> Result<Entity> {
		let entity = world.spawn_empty().id();
		for (ident, value) in components {
			if let Err(err) =
				Self::insert(world, entity, &ident, value, exposure)
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

	/// The schema `ident` declares, having checked this exposure may write it.
	///
	/// The read half of a write: the short exclusive section an async operation
	/// runs before validating, so the validation itself happens with no world
	/// access held. [`None`] means nothing to check: a registered component,
	/// whose contract is its rust type, or a runtime one declared
	/// [`Any`](ValueSchema::Any).
	///
	/// # Errors
	/// Errors when the identifier is unknown or the exposure excludes it, which
	/// is why this runs before the value is looked at rather than after.
	pub fn declared_schema(
		world: &mut World,
		ident: &str,
		exposure: &ScriptExposure,
	) -> Result<Option<ValueSchema>> {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_writable(&ident)?;
		let Some(schema) = DynamicComponents::schema_of(world, ident.id) else {
			return None.xok();
		};
		if let ValueSchema::Any = schema {
			return None.xok();
		}
		let schema = schema.clone();
		// a `Reference` names a schema the registry holds, so it is resolved
		// here, where the world is available, rather than deferred to a
		// validation that would treat it as a wildcard.
		world
			.get_resource::<SchemaRegistry>()
			.map(|registry| registry.resolve(&schema))
			.unwrap_or(schema)
			.xmap(Some)
			.xok()
	}

	/// Check `value` against `schema`, coercing it where the schema says to.
	///
	/// Async because a schema is allowed to ask something beyond the world
	/// before it will accept a value ("is this transaction id valid" can mean
	/// "is there enough money in the account"), which is why the operation that
	/// calls this is async too.
	///
	/// # Errors
	/// Errors naming the component and every failing path, as one line, so the
	/// script catches a schema failure exactly the way it catches an exposure
	/// refusal.
	pub async fn validate(
		schema: &ValueSchema,
		ident: &str,
		value: &mut Value,
	) -> Result {
		let errors = schema.validate(value).await;
		if errors.is_empty() {
			return Ok(());
		}
		bevybail!(
			"`{ident}` does not accept this value: {}",
			errors
				.iter()
				.map(ToString::to_string)
				.collect::<Vec<_>>()
				.join("; ")
		)
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

	/// One component, as the map a spawn takes.
	fn components(ident: &str, value: impl Into<Value>) -> Map {
		let mut components = Map::default();
		components.insert(ident, value);
		components
	}

	#[beet_core::test]
	fn inserts_a_registered_component() {
		let mut world = test_world();
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"Name",
			Value::from("ada"),
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
			components("Name", "ada"),
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
			components("Nonesuch", 1u64),
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
			Value::from("ada"),
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
			Value::from("bob"),
			&ScriptExposure::default(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("was despawned");
	}

	/// A registered component has no declared schema: its rust type is its
	/// contract, checked by the reflect deserializer at the insert.
	#[beet_core::test]
	fn a_registered_component_declares_no_schema() {
		let mut world = test_world();
		WorldWrite::declared_schema(
			&mut world,
			"Name",
			&ScriptExposure::default(),
		)
		.unwrap()
		.is_none()
		.xpect_true();
	}

	/// The default is inert: an undeclared runtime component has nothing to
	/// check, so the write path skips validation entirely.
	#[beet_core::test]
	fn an_any_schema_is_nothing_to_check() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot", ValueSchema::Any)
			.unwrap();
		WorldWrite::declared_schema(
			&mut world,
			"game.Loot",
			&ScriptExposure::default(),
		)
		.unwrap()
		.is_none()
		.xpect_true();
	}

	#[beet_core::test]
	async fn a_rejected_value_names_the_component() {
		WorldWrite::validate(
			&ValueSchema::U64(default()),
			"game.Health",
			&mut Value::from("not a number"),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("`game.Health` does not accept this value");
	}
}
