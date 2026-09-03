//! The one place a script's reads leave the world.
//!
//! The mirror of [`WorldWrite`]: three operations, each checked against the
//! config's read filter before it looks at anything, each serving from the
//! world as it is at the moment of the call.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::reflect::TypeInfo;
use bevy::reflect::serde::TypedReflectSerializer;
use serde::Serialize;

/// The three world reads a script can express.
pub struct WorldRead;

impl WorldRead {
	/// Serialize `ident` off `entity`, absent when the entity does not carry it.
	///
	/// # Errors
	/// Errors when the identifier is unknown, the config excludes it, the
	/// entity is gone, or the component does not serialize.
	pub fn get(
		world: &mut World,
		entity: Entity,
		ident: &str,
		config: &ScriptConfig,
	) -> Result<Option<Value>> {
		let ident = ComponentIdent::resolve(world, ident)?;
		config.assert_readable(&ident)?;
		let entity = world.get_entity(entity).map_err(|_| {
			bevyhow!("{entity} was despawned before the script read it")
		})?;
		match ident.type_id {
			// SAFETY: `DynamicComponents::register` is the only source of a
			// dynamic `ComponentId`, and it always declares a `Value` layout.
			None => entity
				.get_by_id(ident.id)
				.ok()
				.map(|ptr| unsafe { ptr.deref::<Value>() }.clone())
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
				// straight into a `Value`, the currency the whole bridge speaks:
				// `ValueSerializer` is a real serde data format, so a registered
				// component needs no JSON hop to reach the wire.
				TypedReflectSerializer::new(
					value.as_partial_reflect(),
					&registry,
				)
				.serialize(ValueSerializer)
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
	/// Errors when the identifier is unknown or the config excludes it.
	pub fn entities(
		world: &mut World,
		ident: &str,
		config: &ScriptConfig,
	) -> Result<Value> {
		let ident = ComponentIdent::resolve(world, ident)?;
		config.assert_readable(&ident)?;
		QueryBuilder::<Entity>::new(world)
			.with_id(ident.id)
			.build()
			.iter(world)
			.map(|entity| Value::str(entity_id::encode(entity)))
			.collect::<Vec<_>>()
			.xmap(Value::List)
			.xok()
	}

	/// A structural description of `ident`.
	///
	/// The two kinds of component answer differently, and the difference is
	/// real. A runtime component answers with the [`ValueSchema`] its
	/// [`DynamicComponent`] declared, which *is* a contract: every write to it
	/// is validated against exactly this. A registered component answers with a
	/// shallow sketch of its reflected [`TypeInfo`], the kind and one level of
	/// field names and type paths, which stays a courtesy for a script author
	/// working against vocabulary they did not write and which nothing validates
	/// against.
	///
	/// # Errors
	/// Errors when the identifier is unknown or the config excludes it.
	pub fn schema(
		world: &mut World,
		ident: &str,
		config: &ScriptConfig,
	) -> Result<Value> {
		let ident = ComponentIdent::resolve(world, ident)?;
		config.assert_readable(&ident)?;
		let Some(type_id) = ident.type_id else {
			// a runtime component's declaration is its whole definition, so its
			// declared schema is the honest and complete answer.
			let schema = DynamicComponents::schema_of(world, ident.id)
				.cloned()
				.unwrap_or(ValueSchema::Any);
			return Self::map([
				("component", Value::str(ident.path.as_str())),
				("schema", Value::from_serde(&schema)?),
			])
			.xok();
		};
		let registry = world.resource::<AppTypeRegistry>().read();
		let info = registry
			.get(type_id)
			.ok_or_else(|| bevyhow!("`{}` left the registry", ident.path))?
			.type_info();
		let mut schema = Self::describe(info);
		schema
			.as_map_mut()
			.map_err(|err| bevyhow!("{err}"))?
			.insert("component", ident.path.as_str());
		schema.xok()
	}

	/// A [`Value::Map`] from its entries, the shape every description takes.
	fn map<const N: usize>(entries: [(&str, Value); N]) -> Value {
		let mut map = Map::default();
		for (key, value) in entries {
			map.insert(key, value);
		}
		Value::Map(map)
	}

	/// One level of structure for a reflected type.
	fn describe(info: &TypeInfo) -> Value {
		let field = |name: String, type_path: &str| {
			Self::map([
				("name", Value::str(name)),
				("type", Value::str(type_path)),
			])
		};
		let list = |values: Vec<Value>| Value::List(values);
		match info {
			TypeInfo::Struct(info) => Self::map([
				("kind", Value::str("struct")),
				(
					"fields",
					list(
						info.iter()
							.map(|info| {
								field(info.name().to_string(), info.type_path())
							})
							.collect(),
					),
				),
			]),
			TypeInfo::TupleStruct(info) => Self::map([
				("kind", Value::str("tuple_struct")),
				(
					"fields",
					list(
						info.iter()
							.enumerate()
							.map(|(index, info)| {
								field(index.to_string(), info.type_path())
							})
							.collect(),
					),
				),
			]),
			TypeInfo::Enum(info) => Self::map([
				("kind", Value::str("enum")),
				(
					"variants",
					list(
						info.iter()
							.map(|variant| Value::str(variant.name()))
							.collect(),
					),
				),
			]),
			TypeInfo::List(info) => Self::map([
				("kind", Value::str("list")),
				("item", Value::str(info.item_ty().path())),
			]),
			TypeInfo::Array(info) => Self::map([
				("kind", Value::str("array")),
				("item", Value::str(info.item_ty().path())),
				("length", Value::Uint(info.capacity() as u64)),
			]),
			TypeInfo::Map(info) => Self::map([
				("kind", Value::str("map")),
				("key", Value::str(info.key_ty().path())),
				("value", Value::str(info.value_ty().path())),
			]),
			TypeInfo::Set(info) => Self::map([
				("kind", Value::str("set")),
				("item", Value::str(info.value_ty().path())),
			]),
			// a tuple or an opaque type (a number, a string, a `Name`) has no
			// structure worth describing; the kind alone is the honest answer.
			other => Self::map([("kind", Value::str(Self::kind(other)))]),
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
		WorldRead::get(&mut world, entity, "Name", &ScriptConfig::default())
			.unwrap()
			.unwrap()
			.xpect_eq(Value::from("ada"));
	}

	#[beet_core::test]
	fn reads_a_dynamic_component() {
		let mut world = test_world();
		DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::Any,
		)
		.unwrap();
		let entity = world.spawn_empty().id();
		WorldWrite::insert(
			&mut world,
			entity,
			"game.Health",
			Value::Uint(3),
			&ScriptConfig::default(),
		)
		.unwrap();
		WorldRead::get(
			&mut world,
			entity,
			"game.Health",
			&ScriptConfig::default(),
		)
		.unwrap()
		.unwrap()
		.xpect_eq(Value::Uint(3));
	}

	#[beet_core::test]
	fn lists_the_entities_carrying_a_component() {
		let mut world = test_world();
		world.spawn_empty();
		world.spawn(Name::new("ada"));
		world.spawn(Name::new("bob"));
		WorldRead::entities(&mut world, "Name", &ScriptConfig::default())
			.unwrap()
			.as_list()
			.unwrap()
			.len()
			.xpect_eq(2);
	}

	#[beet_core::test]
	fn a_read_outside_the_config_names_the_path() {
		let mut world = test_world();
		let entity = world.spawn(Name::new("ada")).id();
		WorldRead::get(
			&mut world,
			entity,
			"Name",
			&ScriptConfig::new(["game.Health"]),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("may not read `bevy_ecs::name::Name`");
	}

	#[beet_core::test]
	fn describes_a_registered_component() {
		let mut world = test_world();
		WorldRead::schema(&mut world, "Name", &ScriptConfig::default())
			.unwrap()
			.to_string()
			.xpect_contains("bevy_ecs::name::Name")
			.xpect_contains("kind");
	}

	/// A runtime component's schema is its declaration, so `world.schema`
	/// answers with the contract every write to it is checked against.
	#[beet_core::test]
	fn describes_a_dynamic_component() {
		let mut world = test_world();
		DynamicComponents::register(
			&mut world,
			"game.Health",
			ValueSchema::U64(default()),
		)
		.unwrap();
		WorldRead::schema(&mut world, "game.Health", &ScriptConfig::default())
			.unwrap()
			.xpect_eq(WorldRead::map([
				("component", Value::str("game.Health")),
				(
					"schema",
					Value::from_serde(&ValueSchema::U64(default())).unwrap(),
				),
			]));
	}

	/// An undeclared runtime component is `Any`, so its schema says so rather
	/// than pretending to a shape.
	#[beet_core::test]
	fn an_undeclared_dynamic_component_is_any() {
		let mut world = test_world();
		DynamicComponents::register(&mut world, "game.Loot", ValueSchema::Any)
			.unwrap();
		WorldRead::schema(&mut world, "game.Loot", &ScriptConfig::default())
			.unwrap()
			.to_string()
			.xpect_contains("Any");
	}
}
