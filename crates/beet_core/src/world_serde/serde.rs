//! `serde` serialization and deserialization for [`DynamicWorld`].
//!
//! Unlike the upstream `bevy_world_serialization`, this implementation does not handle
//! asset handles, dropping the dependency on `bevy_asset` and keeping the module `no_std`.

use super::DynamicEntity;
use super::DynamicWorld;
use crate::prelude::*;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeRegistry;
use bevy_reflect::serde::ReflectDeserializer;
use bevy_reflect::serde::TypeRegistrationDeserializer;
use bevy_reflect::serde::TypedReflectDeserializer;
use bevy_reflect::serde::TypedReflectSerializer;
use core::fmt::Formatter;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use serde::de::DeserializeSeed;
use serde::de::Error;
use serde::de::MapAccess;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeMap;
use serde::ser::SerializeStruct;

/// Name of the serialized world struct type.
pub const WORLD_STRUCT: &str = "World";
/// Name of the serialized resources field in a world struct.
pub const WORLD_RESOURCES: &str = "resources";
/// Name of the serialized entities field in a world struct.
pub const WORLD_ENTITIES: &str = "entities";

/// Name of the serialized entity struct type.
pub const ENTITY_STRUCT: &str = "Entity";
/// Name of the serialized component field in an entity struct.
pub const ENTITY_FIELD_COMPONENTS: &str = "components";

/// Serializer for a [`DynamicWorld`], implementing [`Serialize`] for use with Serde.
pub struct DynamicWorldSerializer<'a> {
	/// The dynamic world to serialize.
	pub world: &'a DynamicWorld,
	/// The type registry containing the types present in the dynamic world.
	pub registry: &'a TypeRegistry,
}

impl<'a> DynamicWorldSerializer<'a> {
	/// Create a new serializer from a [`DynamicWorld`] and an associated [`TypeRegistry`].
	///
	/// The type registry must contain all types present in the [`DynamicWorld`].
	pub fn new(world: &'a DynamicWorld, registry: &'a TypeRegistry) -> Self {
		DynamicWorldSerializer { world, registry }
	}
}

impl Serialize for DynamicWorldSerializer<'_> {
	fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct(WORLD_STRUCT, 2)?;
		state.serialize_field(
			WORLD_RESOURCES,
			&WorldMapSerializer {
				entries: &self.world.resources,
				registry: self.registry,
			},
		)?;
		state.serialize_field(
			WORLD_ENTITIES,
			&EntitiesSerializer {
				entities: &self.world.entities,
				registry: self.registry,
			},
		)?;
		state.end()
	}
}

/// Serializes multiple entities as a map of entity id to serialized entity.
pub struct EntitiesSerializer<'a> {
	/// The entities to serialize.
	pub entities: &'a [DynamicEntity],
	/// Type registry in which the entities' component types are registered.
	pub registry: &'a TypeRegistry,
}

impl Serialize for EntitiesSerializer<'_> {
	fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_map(Some(self.entities.len()))?;
		for entity in self.entities {
			state.serialize_entry(
				&entity.entity,
				&EntitySerializer {
					entity,
					registry: self.registry,
				},
			)?;
		}
		state.end()
	}
}

/// Serializes an entity as a map of component type to component value.
pub struct EntitySerializer<'a> {
	/// The entity to serialize.
	pub entity: &'a DynamicEntity,
	/// Type registry in which the entity's component types are registered.
	pub registry: &'a TypeRegistry,
}

impl Serialize for EntitySerializer<'_> {
	fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct(ENTITY_STRUCT, 1)?;
		state.serialize_field(
			ENTITY_FIELD_COMPONENTS,
			&WorldMapSerializer {
				entries: &self.entity.components,
				registry: self.registry,
			},
		)?;
		state.end()
	}
}

/// Serializes a list of unique-typed values as a map of type path to value.
///
/// Used for world resources and entity components. The entries are sorted by type path before
/// serialization.
pub struct WorldMapSerializer<'a> {
	/// List of boxed values of unique type to serialize.
	pub entries: &'a [Box<dyn PartialReflect>],
	/// Type registry in which the `entries` types are registered.
	pub registry: &'a TypeRegistry,
}

impl Serialize for WorldMapSerializer<'_> {
	fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_map(Some(self.entries.len()))?;
		let sorted_entries = {
			let mut entries = self
				.entries
				.iter()
				.map(|entry| {
					(
						entry.get_represented_type_info().unwrap().type_path(),
						entry.as_partial_reflect(),
					)
				})
				.collect::<Vec<_>>();
			entries.sort_by_key(|(type_path, _)| *type_path);
			entries
		};

		for (type_path, partial_reflect) in sorted_entries {
			state.serialize_entry(
				type_path,
				&TypedReflectSerializer::new(partial_reflect, self.registry),
			)?;
		}
		state.end()
	}
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum WorldField {
	Resources,
	Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
	Components,
}

/// Handles world deserialization into a [`DynamicWorld`].
pub struct WorldDeserializer<'a> {
	/// Type registry in which the world's components and resources are registered.
	pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for WorldDeserializer<'a> {
	type Value = DynamicWorld;

	fn deserialize<D>(self, deserializer: D) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_struct(
			WORLD_STRUCT,
			&[WORLD_RESOURCES, WORLD_ENTITIES],
			WorldVisitor {
				type_registry: self.type_registry,
			},
		)
	}
}

struct WorldVisitor<'a> {
	type_registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for WorldVisitor<'_> {
	type Value = DynamicWorld;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("world struct")
	}

	fn visit_seq<A>(self, mut seq: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let resources = seq
			.next_element_seed(WorldMapDeserializer {
				registry: self.type_registry,
			})?
			.ok_or_else(|| Error::missing_field(WORLD_RESOURCES))?;

		let entities = seq
			.next_element_seed(WorldEntitiesDeserializer {
				type_registry: self.type_registry,
			})?
			.ok_or_else(|| Error::missing_field(WORLD_ENTITIES))?;

		Ok(DynamicWorld {
			resources,
			entities,
		})
	}

	fn visit_map<A>(self, mut map: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut resources = None;
		let mut entities = None;
		while let Some(key) = map.next_key()? {
			match key {
				WorldField::Resources => {
					if resources.is_some() {
						return Err(Error::duplicate_field(WORLD_RESOURCES));
					}
					resources = Some(map.next_value_seed(WorldMapDeserializer {
						registry: self.type_registry,
					})?);
				}
				WorldField::Entities => {
					if entities.is_some() {
						return Err(Error::duplicate_field(WORLD_ENTITIES));
					}
					entities = Some(map.next_value_seed(WorldEntitiesDeserializer {
						type_registry: self.type_registry,
					})?);
				}
			}
		}

		let resources =
			resources.ok_or_else(|| Error::missing_field(WORLD_RESOURCES))?;
		let entities =
			entities.ok_or_else(|| Error::missing_field(WORLD_ENTITIES))?;

		Ok(DynamicWorld {
			resources,
			entities,
		})
	}
}

/// Handles deserialization for a collection of entities.
pub struct WorldEntitiesDeserializer<'a> {
	/// Type registry in which the entities' component types are registered.
	pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for WorldEntitiesDeserializer<'a> {
	type Value = Vec<DynamicEntity>;

	fn deserialize<D>(self, deserializer: D) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_map(WorldEntitiesVisitor {
			type_registry: self.type_registry,
		})
	}
}

struct WorldEntitiesVisitor<'a> {
	type_registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for WorldEntitiesVisitor<'_> {
	type Value = Vec<DynamicEntity>;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("map of entities")
	}

	fn visit_map<A>(self, mut map: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut entities = Vec::new();
		while let Some(entity) = map.next_key::<Entity>()? {
			entities.push(map.next_value_seed(WorldEntityDeserializer {
				entity,
				type_registry: self.type_registry,
			})?);
		}
		Ok(entities)
	}
}

/// Handles deserialization of an entity and its components.
pub struct WorldEntityDeserializer<'a> {
	/// Id of the deserialized entity.
	pub entity: Entity,
	/// Type registry in which the entity's component types are registered.
	pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for WorldEntityDeserializer<'a> {
	type Value = DynamicEntity;

	fn deserialize<D>(self, deserializer: D) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_struct(
			ENTITY_STRUCT,
			&[ENTITY_FIELD_COMPONENTS],
			WorldEntityVisitor {
				entity: self.entity,
				registry: self.type_registry,
			},
		)
	}
}

struct WorldEntityVisitor<'a> {
	entity: Entity,
	registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for WorldEntityVisitor<'_> {
	type Value = DynamicEntity;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("entities")
	}

	fn visit_seq<A>(self, mut seq: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let components = seq
			.next_element_seed(WorldMapDeserializer {
				registry: self.registry,
			})?
			.ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;

		Ok(DynamicEntity {
			entity: self.entity,
			components,
		})
	}

	fn visit_map<A>(self, mut map: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut components = None;
		while let Some(key) = map.next_key()? {
			match key {
				EntityField::Components => {
					if components.is_some() {
						return Err(Error::duplicate_field(ENTITY_FIELD_COMPONENTS));
					}
					components = Some(map.next_value_seed(WorldMapDeserializer {
						registry: self.registry,
					})?);
				}
			}
		}

		let components = components
			.take()
			.ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
		Ok(DynamicEntity {
			entity: self.entity,
			components,
		})
	}
}

/// Handles deserialization of a sequence of values with unique types.
pub struct WorldMapDeserializer<'a> {
	/// Type registry in which the values' types are registered.
	pub registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for WorldMapDeserializer<'a> {
	type Value = Vec<Box<dyn PartialReflect>>;

	fn deserialize<D>(self, deserializer: D) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_map(WorldMapVisitor {
			registry: self.registry,
		})
	}
}

struct WorldMapVisitor<'a> {
	registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for WorldMapVisitor<'_> {
	type Value = Vec<Box<dyn PartialReflect>>;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("map of reflect types")
	}

	fn visit_seq<A>(self, mut seq: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let mut dynamic_properties = Vec::new();
		while let Some(entity) =
			seq.next_element_seed(ReflectDeserializer::new(self.registry))?
		{
			dynamic_properties.push(entity);
		}
		Ok(dynamic_properties)
	}

	fn visit_map<A>(self, mut map: A) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut added = <HashSet<_>>::default();
		let mut entries = Vec::new();
		while let Some(registration) =
			map.next_key_seed(TypeRegistrationDeserializer::new(self.registry))?
		{
			if !added.insert(registration.type_id()) {
				return Err(Error::custom(format_args!(
					"duplicate reflect type: `{}`",
					registration.type_info().type_path(),
				)));
			}

			let value = map.next_value_seed(TypedReflectDeserializer::new(
				registration,
				self.registry,
			))?;

			// attempt to convert using FromReflect
			let value = self
				.registry
				.get(registration.type_id())
				.and_then(|registration| registration.data::<ReflectFromReflect>())
				.and_then(|from_reflect| {
					from_reflect.from_reflect(value.as_partial_reflect())
				})
				.map(PartialReflect::into_partial_reflect)
				.unwrap_or(value);

			entries.push(value);
		}

		Ok(entries)
	}
}

#[cfg(test)]
mod test {
	use super::DynamicWorldSerializer;
	use super::WorldDeserializer;
	use crate::prelude::*;
	use bevy::ecs::entity::EntityHashMap;
	use serde::Deserialize;
	use serde::Serialize;
	use serde::de::DeserializeSeed;

	// de/serialize as hex, to exercise the custom serde path
	mod qux {
		use crate::prelude::*;
		use serde::Deserializer;
		use serde::Serializer;
		use serde::de::Error;

		pub fn serialize<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			serializer.serialize_str(&format!("{value:X}"))
		}

		pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
		where
			D: Deserializer<'de>,
		{
			u32::from_str_radix(
				<&str as serde::Deserialize>::deserialize(deserializer)?,
				16,
			)
			.map_err(Error::custom)
		}
	}

	#[derive(Component, Copy, Clone, Reflect, Debug, PartialEq, Serialize, Deserialize)]
	#[reflect(Component, Serialize, Deserialize)]
	struct Qux(#[serde(with = "qux")] u32);

	#[derive(Component, Reflect, Default, PartialEq, Debug)]
	#[reflect(Component)]
	struct MyComponent {
		foo: [usize; 3],
		bar: (f32, f32),
		baz: MyEnum,
	}

	#[derive(Reflect, Default, PartialEq, Debug)]
	enum MyEnum {
		#[default]
		Unit,
		Tuple(String),
		Struct {
			value: u32,
		},
	}

	fn create_world() -> World {
		let mut world = World::new();
		let registry = AppTypeRegistry::default();
		{
			let mut registry = registry.write();
			registry.register::<Qux>();
			registry.register::<MyComponent>();
			registry.register::<MyEnum>();
			registry.register::<String>();
			registry.register_type_data::<String, bevy_reflect::ReflectSerialize>();
			registry.register::<[usize; 3]>();
			registry.register::<(f32, f32)>();
		}
		world.insert_resource(registry);
		world
	}

	/// Serialize then deserialize the world, returning the deserialized [`DynamicWorld`].
	#[cfg(feature = "ron")]
	fn roundtrip_ron(world: &World) -> DynamicWorld {
		let dynamic_world = DynamicWorld::from_world(world);
		let registry = world.resource::<AppTypeRegistry>().read();
		let serialized = ron::ser::to_string(&DynamicWorldSerializer::new(
			&dynamic_world,
			&registry,
		))
		.unwrap();
		WorldDeserializer {
			type_registry: &registry,
		}
		.deserialize(&mut ron::de::Deserializer::from_str(&serialized).unwrap())
		.unwrap()
	}

	#[cfg(feature = "ron")]
	#[crate::test]
	fn roundtrips_custom_serialization() {
		let mut world = create_world();
		world.spawn(Qux(42));

		let deserialized = roundtrip_ron(&world);
		deserialized.entities.len().xpect_eq(1);

		let mut world = create_world();
		deserialized
			.write_to_world(&mut world, &mut EntityHashMap::default())
			.unwrap();
		world.query::<&Qux>().single(&world).unwrap().xpect_eq(Qux(42));
	}

	#[cfg(feature = "postcard")]
	#[crate::test]
	fn roundtrips_postcard() {
		let mut world = create_world();
		world.spawn(MyComponent {
			foo: [1, 2, 3],
			bar: (1.3, 3.7),
			baz: MyEnum::Tuple("Hello World!".to_string()),
		});

		let registry = world.resource::<AppTypeRegistry>().read();
		let dynamic_world = DynamicWorld::from_world(&world);
		let serialized = postcard::to_allocvec(&DynamicWorldSerializer::new(
			&dynamic_world,
			&registry,
		))
		.unwrap();

		let deserialized = WorldDeserializer {
			type_registry: &registry,
		}
		.deserialize(&mut postcard::Deserializer::from_bytes(&serialized))
		.unwrap();

		deserialized.entities.len().xpect_eq(1);
		deserialized.entities[0].components[0]
			.try_as_reflect()
			.unwrap()
			.downcast_ref::<MyComponent>()
			.unwrap()
			.xpect_eq(MyComponent {
				foo: [1, 2, 3],
				bar: (1.3, 3.7),
				baz: MyEnum::Tuple("Hello World!".to_string()),
			});
	}
}
