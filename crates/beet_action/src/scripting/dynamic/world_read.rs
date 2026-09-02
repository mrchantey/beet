//! The one place a script's reads leave the world.
//!
//! The mirror of [`WorldWrite`]: three operations, each checked against the
//! exposure's read filter before it looks at anything, each serving from the
//! world as it is at the moment of the call.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::serde::TypedReflectSerializer;
use serde_json::Value as JsonValue;

/// The three world reads a script can express.
pub struct WorldRead;

impl WorldRead {
	/// Serialize `ident` off `entity`, absent when the entity does not carry it.
	///
	/// # Errors
	/// Errors when the identifier is unknown, the exposure excludes it, the
	/// entity is gone, or the component does not serialize.
	pub fn get(
		world: &mut World,
		entity: Entity,
		ident: &str,
		exposure: &ScriptExposure,
	) -> Result<Option<JsonValue>> {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_readable(&ident)?;
		let entity = world.get_entity(entity).map_err(|_| {
			bevyhow!("{entity} was despawned before the script read it")
		})?;
		match ident.type_id {
			// SAFETY: `DynamicComponents::register` is the only source of a
			// dynamic `ComponentId`, and it always declares a `Value` layout.
			None => entity
				.get_by_id(ident.id)
				.ok()
				.map(|ptr| unsafe { ptr.deref::<Value>() }.clone().into_json())
				.xok(),
			Some(type_id) => {
				let registry = world.resource::<AppTypeRegistry>().read();
				let Some(reflect_component) =
					registry.get(type_id).and_then(|registration| {
						registration.data::<ReflectComponent>()
					})
				else {
					return None.xok();
				};
				let Some(value) = reflect_component.reflect(entity) else {
					return None.xok();
				};
				serde_json::to_value(TypedReflectSerializer::new(
					value.as_partial_reflect(),
					&registry,
				))
				.map_err(|err| {
					bevyhow!("failed to serialize `{}`: {err}", ident.path)
				})
				.map(Some)
			}
		}
	}

	/// The id of every entity carrying `ident`.
	///
	/// # Errors
	/// Errors when the identifier is unknown or the exposure excludes it.
	pub fn entities(
		world: &mut World,
		ident: &str,
		exposure: &ScriptExposure,
	) -> Result<JsonValue> {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_readable(&ident)?;
		QueryBuilder::<Entity>::new(world)
			.with_id(ident.id)
			.build()
			.iter(world)
			.map(|entity| JsonValue::String(entity_id::encode(entity)))
			.collect::<Vec<_>>()
			.xmap(JsonValue::Array)
			.xok()
	}

	/// A loose structural description of `ident`, derived from its reflected
	/// [`TypeInfo`].
	///
	/// A courtesy for a script author working against vocabulary they did not
	/// write, deliberately shallow: the kind, and one level of field names and
	/// type paths. It is not a contract, and nothing downstream validates
	/// against it.
	///
	/// # Errors
	/// Errors when the identifier is unknown or the exposure excludes it.
	pub fn schema(
		world: &mut World,
		ident: &str,
		exposure: &ScriptExposure,
	) -> Result<JsonValue> {
		let ident = ComponentIdent::resolve(world, ident)?;
		exposure.assert_readable(&ident)?;
		let Some(type_id) = ident.type_id else {
			// a runtime component has no rust type behind it, and its value is
			// whatever a document field can hold, so that is the honest answer.
			return serde_json::json!({
				"component": ident.path,
				"kind": "dynamic",
			})
			.xok();
		};
		let registry = world.resource::<AppTypeRegistry>().read();
		let info = registry
			.get(type_id)
			.ok_or_else(|| bevyhow!("`{}` left the registry", ident.path))?
			.type_info();
		let mut schema = Self::describe(info);
		schema["component"] = JsonValue::String(ident.path.to_string());
		schema.xok()
	}

	/// One level of structure for a reflected type.
	fn describe(info: &TypeInfo) -> JsonValue {
		let field = |name: String, type_path: &str| serde_json::json!({ "name": name, "type": type_path });
		match info {
			TypeInfo::Struct(info) => serde_json::json!({
				"kind": "struct",
				"fields": info
					.iter()
					.map(|info| field(info.name().to_string(), info.type_path()))
					.collect::<Vec<_>>(),
			}),
			TypeInfo::TupleStruct(info) => serde_json::json!({
				"kind": "tuple_struct",
				"fields": info
					.iter()
					.enumerate()
					.map(|(index, info)| field(index.to_string(), info.type_path()))
					.collect::<Vec<_>>(),
			}),
			TypeInfo::Enum(info) => serde_json::json!({
				"kind": "enum",
				"variants": info
					.iter()
					.map(|variant| variant.name())
					.collect::<Vec<_>>(),
			}),
			TypeInfo::List(info) => serde_json::json!({
				"kind": "list",
				"item": info.item_ty().path(),
			}),
			TypeInfo::Array(info) => serde_json::json!({
				"kind": "array",
				"item": info.item_ty().path(),
				"length": info.capacity(),
			}),
			TypeInfo::Map(info) => serde_json::json!({
				"kind": "map",
				"key": info.key_ty().path(),
				"value": info.value_ty().path(),
			}),
			TypeInfo::Set(info) => serde_json::json!({
				"kind": "set",
				"item": info.value_ty().path(),
			}),
			// a tuple or an opaque type (a number, a string, a `Name`) has no
			// structure worth describing; the kind alone is the honest answer.
			other => serde_json::json!({ "kind": Self::kind(other) }),
		}
	}

	/// The one-word name of a [`TypeInfo`] kind.
	fn kind(info: &TypeInfo) -> &'static str {
		match info {
			TypeInfo::Struct(_) => "struct",
			TypeInfo::TupleStruct(_) => "tuple_struct",
			TypeInfo::Tuple(_) => "tuple",
			TypeInfo::List(_) => "list",
			TypeInfo::Array(_) => "array",
			TypeInfo::Map(_) => "map",
			TypeInfo::Set(_) => "set",
			TypeInfo::Enum(_) => "enum",
			TypeInfo::Opaque(_) => "opaque",
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use crate::scripting::dynamic::test_support::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	fn reads_a_registered_component() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		WorldRead::get(&mut world, entity, "Name", &ScriptExposure::default())
			.unwrap()
			.unwrap()
			.xpect_eq(serde_json::json!("ada"));
	}

	#[beet_core::test]
	fn reads_a_dynamic_component() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Health");
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"game.Health",
			&serde_json::json!(3),
			&ScriptExposure::default(),
		)
		.unwrap();
		WorldRead::get(
			&mut world,
			entity,
			"game.Health",
			&ScriptExposure::default(),
		)
		.unwrap()
		.unwrap()
		.xpect_eq(serde_json::json!(3));
	}

	#[beet_core::test]
	fn lists_the_entities_carrying_a_component() {
		let mut world = test_world();
		world.spawn_empty();
		world.spawn(Name::new("ada"));
		world.spawn(Name::new("bob"));
		WorldRead::entities(&mut world, "Name", &ScriptExposure::default())
			.unwrap()
			.as_array()
			.unwrap()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn a_read_outside_the_exposure_names_the_path() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		WorldRead::get(
			&mut world,
			entity,
			"Name",
			&ScriptExposure::new(["game.Health"]),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("may not read `bevy_ecs::name::Name`");
	}

	#[beet_core::test]
	fn describes_a_registered_component() {
		let mut world = test_world();
		WorldRead::schema(&mut world, "Name", &ScriptExposure::default())
			.unwrap()
			.to_string()
			.xpect_contains(r#""component":"bevy_ecs::name::Name""#)
			.xpect_contains(r#""kind":"#);
	}

	/// A runtime component has no rust type behind it, so its schema says so
	/// rather than inventing a shape.
	#[beet_core::test]
	fn describes_a_dynamic_component() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Health");
		WorldRead::schema(
			&mut world,
			"game.Health",
			&ScriptExposure::default(),
		)
		.unwrap()
		.xpect_eq(serde_json::json!({
			"component": "game.Health",
			"kind": "dynamic",
		}));
	}
}
