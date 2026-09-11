//! `serde` serialization and deserialization for [`DynamicTemplate`].
//!
//! # Serializer mode: resolved values
//!
//! [`DynamicTemplateSerializer`] is the save-game serializer: it writes a
//! [`DynamicTemplate`] whose component slots are resolved values
//! ([`ComponentSlot::Value`]). A deferred-template slot
//! ([`ComponentSlot::Template`]) is an authoring construct, not a resolved value,
//! so serializing one is an error rather than a silent drop. An authoring
//! snapshot that carries a template's identity (name plus patch) is the parser's
//! concern, not this reflect-value path.
//!
//! Deserialization is symmetric: every component is read back as a value slot.
//! No asset type is referenced, keeping this module no_std.

use super::ComponentSlot;
use super::DynamicTemplate;
use super::DynamicTemplateEntity;
use crate::prelude::*;
use bevy_reflect::PartialReflect;
use bevy_reflect::ReflectFromReflect;
use bevy_reflect::TypeRegistration;
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

/// Name of the serialized template struct type.
pub(crate) const TEMPLATE_STRUCT: &str = "Template";
/// Name of the serialized resources field in a template struct.
pub(crate) const TEMPLATE_RESOURCES: &str = "resources";
/// Name of the serialized entities field in a template struct.
pub(crate) const TEMPLATE_ENTITIES: &str = "entities";

/// Name of the serialized entity struct type.
pub(crate) const ENTITY_STRUCT: &str = "Entity";
/// Name of the serialized components field in an entity struct.
pub(crate) const ENTITY_FIELD_COMPONENTS: &str = "components";

/// Serializer for a [`DynamicTemplate`] of resolved values.
///
/// See the [module docs](self) for the resolved-value serializer mode.
pub struct DynamicTemplateSerializer<'a> {
	/// The template to serialize.
	pub template: &'a DynamicTemplate,
	/// The type registry containing the types present in the template.
	pub registry: &'a TypeRegistry,
}

impl<'a> DynamicTemplateSerializer<'a> {
	/// Create a serializer from a [`DynamicTemplate`] and its [`TypeRegistry`].
	///
	/// The registry must contain all types present in the template.
	pub fn new(
		template: &'a DynamicTemplate,
		registry: &'a TypeRegistry,
	) -> Self {
		DynamicTemplateSerializer { template, registry }
	}
}

impl Serialize for DynamicTemplateSerializer<'_> {
	fn serialize<S>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct(TEMPLATE_STRUCT, 2)?;
		state.serialize_field(TEMPLATE_RESOURCES, &ValueMapSerializer {
			values: &self.template.resources,
			registry: self.registry,
		})?;
		state.serialize_field(TEMPLATE_ENTITIES, &EntitiesSerializer {
			entities: &self.template.entities,
			registry: self.registry,
		})?;
		state.end()
	}
}

/// Serializes entities as a map of file key to serialized entity.
///
/// A file key is its in-template [`Entity`] with the generation stripped, so a
/// file reads `0`, `1`, `2` rather than raw 64-bit entity bits. Which key an entity
/// gets is the saver's business ([`TemplateEntityMap`]); this is only the
/// encoding. File *order* is the children-order contract and is emitted as-is.
struct EntitiesSerializer<'a> {
	entities: &'a [DynamicTemplateEntity],
	registry: &'a TypeRegistry,
}

impl Serialize for EntitiesSerializer<'_> {
	fn serialize<S>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_map(Some(self.entities.len()))?;
		for entity in self.entities {
			state.serialize_entry(
				&entity.entity.index_u32(),
				&EntitySerializer {
					entity,
					registry: self.registry,
				},
			)?;
		}
		state.end()
	}
}

/// Serializes an entity as a struct holding its component map.
struct EntitySerializer<'a> {
	entity: &'a DynamicTemplateEntity,
	registry: &'a TypeRegistry,
}

impl Serialize for EntitySerializer<'_> {
	fn serialize<S>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct(ENTITY_STRUCT, 1)?;
		state.serialize_field(ENTITY_FIELD_COMPONENTS, &SlotMapSerializer {
			slots: &self.entity.components,
			registry: self.registry,
		})?;
		state.end()
	}
}

/// Serializes an entity's component slots as a map of type path to value, erroring
/// on a deferred-template slot (see the resolved-value serializer mode).
struct SlotMapSerializer<'a> {
	slots: &'a [ComponentSlot],
	registry: &'a TypeRegistry,
}

impl Serialize for SlotMapSerializer<'_> {
	fn serialize<S>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_map(Some(self.slots.len()))?;
		let mut sorted = self
			.slots
			.iter()
			.map(|slot| match slot {
				ComponentSlot::Value(value) => Ok((
					value
						.get_represented_type_info()
						.ok_or_else(|| {
							<S::Error as serde::ser::Error>::custom(
								"component value has no represented type",
							)
						})?
						.type_path(),
					value.as_partial_reflect(),
				)),
				ComponentSlot::Template(deferred) => {
					Err(<S::Error as serde::ser::Error>::custom(format_args!(
						"cannot serialize a deferred template slot `{}` as a \
						resolved value; resolve it to a value first",
						deferred.name
					)))
				}
			})
			.collect::<core::result::Result<Vec<_>, S::Error>>()?;
		sorted.sort_by_key(|(type_path, _)| *type_path);

		for (type_path, partial_reflect) in sorted {
			state.serialize_entry(
				type_path,
				&TypedReflectSerializer::new(partial_reflect, self.registry),
			)?;
		}
		state.end()
	}
}

/// Serializes a list of unique-typed values (resources) as a map of type path to
/// value, sorted by type path.
struct ValueMapSerializer<'a> {
	values: &'a [Box<dyn PartialReflect>],
	registry: &'a TypeRegistry,
}

impl Serialize for ValueMapSerializer<'_> {
	fn serialize<S>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_map(Some(self.values.len()))?;
		let mut sorted = self
			.values
			.iter()
			.map(|value| {
				(
					value.get_represented_type_info().unwrap().type_path(),
					value.as_partial_reflect(),
				)
			})
			.collect::<Vec<_>>();
		sorted.sort_by_key(|(type_path, _)| *type_path);

		for (type_path, partial_reflect) in sorted {
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
enum TemplateField {
	Resources,
	Entities,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum EntityField {
	Components,
}

/// Deserializes a [`DynamicTemplate`] whose component slots are resolved values.
pub struct DynamicTemplateDeserializer<'a> {
	/// Type registry in which the template's components and resources are
	/// registered.
	pub type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for DynamicTemplateDeserializer<'a> {
	type Value = DynamicTemplate;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_struct(
			TEMPLATE_STRUCT,
			&[TEMPLATE_RESOURCES, TEMPLATE_ENTITIES],
			TemplateVisitor {
				type_registry: self.type_registry,
			},
		)
	}
}

struct TemplateVisitor<'a> {
	type_registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for TemplateVisitor<'_> {
	type Value = DynamicTemplate;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("template struct")
	}

	fn visit_seq<A>(
		self,
		mut seq: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let resources = seq
			.next_element_seed(ValueMapDeserializer {
				registry: self.type_registry,
			})?
			.ok_or_else(|| Error::missing_field(TEMPLATE_RESOURCES))?;

		let entities = seq
			.next_element_seed(EntitiesDeserializer {
				type_registry: self.type_registry,
			})?
			.ok_or_else(|| Error::missing_field(TEMPLATE_ENTITIES))?;

		Ok(DynamicTemplate {
			resources,
			entities,
		})
	}

	fn visit_map<A>(
		self,
		mut map: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut resources = None;
		let mut entities = None;
		while let Some(key) = map.next_key()? {
			match key {
				TemplateField::Resources => {
					if resources.is_some() {
						return Err(Error::duplicate_field(TEMPLATE_RESOURCES));
					}
					resources =
						Some(map.next_value_seed(ValueMapDeserializer {
							registry: self.type_registry,
						})?);
				}
				TemplateField::Entities => {
					if entities.is_some() {
						return Err(Error::duplicate_field(TEMPLATE_ENTITIES));
					}
					entities =
						Some(map.next_value_seed(EntitiesDeserializer {
							type_registry: self.type_registry,
						})?);
				}
			}
		}

		let resources = resources
			.ok_or_else(|| Error::missing_field(TEMPLATE_RESOURCES))?;
		let entities =
			entities.ok_or_else(|| Error::missing_field(TEMPLATE_ENTITIES))?;

		Ok(DynamicTemplate {
			resources,
			entities,
		})
	}
}

/// Deserializes a map of file key to entity.
struct EntitiesDeserializer<'a> {
	type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for EntitiesDeserializer<'a> {
	type Value = Vec<DynamicTemplateEntity>;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_map(EntitiesVisitor {
			type_registry: self.type_registry,
		})
	}
}

struct EntitiesVisitor<'a> {
	type_registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for EntitiesVisitor<'_> {
	type Value = Vec<DynamicTemplateEntity>;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("map of entities")
	}

	fn visit_map<A>(
		self,
		mut map: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut entities = Vec::new();
		while let Some(key) = map.next_key::<u32>()? {
			// a file key is generation-free, so it rehydrates as the in-template
			// entity the build path already keys references on: its index.
			let entity = Entity::from_raw_u32(key).ok_or_else(|| {
				Error::custom(format_args!("`{key}` is not a valid entity key"))
			})?;
			entities.push(map.next_value_seed(EntityDeserializer {
				entity,
				type_registry: self.type_registry,
			})?);
		}
		Ok(entities)
	}
}

/// Deserializes an entity and its component slots.
struct EntityDeserializer<'a> {
	entity: Entity,
	type_registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for EntityDeserializer<'a> {
	type Value = DynamicTemplateEntity;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_struct(
			ENTITY_STRUCT,
			&[ENTITY_FIELD_COMPONENTS],
			EntityVisitor {
				entity: self.entity,
				registry: self.type_registry,
			},
		)
	}
}

struct EntityVisitor<'a> {
	entity: Entity,
	registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for EntityVisitor<'_> {
	type Value = DynamicTemplateEntity;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("entity struct")
	}

	fn visit_seq<A>(
		self,
		mut seq: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let components = seq
			.next_element_seed(SlotMapDeserializer {
				registry: self.registry,
			})?
			.ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;

		Ok(DynamicTemplateEntity {
			entity: self.entity,
			components,
		})
	}

	fn visit_map<A>(
		self,
		mut map: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut components = None;
		while let Some(key) = map.next_key()? {
			match key {
				EntityField::Components => {
					if components.is_some() {
						return Err(Error::duplicate_field(
							ENTITY_FIELD_COMPONENTS,
						));
					}
					components =
						Some(map.next_value_seed(SlotMapDeserializer {
							registry: self.registry,
						})?);
				}
			}
		}

		let components = components
			.ok_or_else(|| Error::missing_field(ENTITY_FIELD_COMPONENTS))?;
		Ok(DynamicTemplateEntity {
			entity: self.entity,
			components,
		})
	}
}

/// Deserializes a map of type path to value into value component slots.
struct SlotMapDeserializer<'a> {
	registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for SlotMapDeserializer<'a> {
	type Value = Vec<ComponentSlot>;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		Ok(ValueMapDeserializer {
			registry: self.registry,
		}
		.deserialize(deserializer)?
		.into_iter()
		.map(ComponentSlot::Value)
		.collect())
	}
}

/// Deserializes a map of type path to value into a list of unique-typed values.
struct ValueMapDeserializer<'a> {
	registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for ValueMapDeserializer<'a> {
	type Value = Vec<Box<dyn PartialReflect>>;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_map(ValueMapVisitor {
			registry: self.registry,
		})
	}
}

struct ValueMapVisitor<'a> {
	registry: &'a TypeRegistry,
}

impl<'de> Visitor<'de> for ValueMapVisitor<'_> {
	type Value = Vec<Box<dyn PartialReflect>>;

	fn expecting(&self, formatter: &mut Formatter) -> core::fmt::Result {
		formatter.write_str("map of reflect types")
	}

	fn visit_seq<A>(
		self,
		mut seq: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: SeqAccess<'de>,
	{
		let mut values = Vec::new();
		while let Some(value) =
			seq.next_element_seed(ReflectDeserializer::new(self.registry))?
		{
			values.push(value);
		}
		Ok(values)
	}

	fn visit_map<A>(
		self,
		mut map: A,
	) -> core::result::Result<Self::Value, A::Error>
	where
		A: MapAccess<'de>,
	{
		let mut added = <HashSet<_>>::default();
		let mut values = Vec::new();
		while let Some(registration) =
			map.next_key_seed(TypeRegistrationDeserializer::new(self.registry))?
		{
			if !added.insert(registration.type_id()) {
				return Err(Error::custom(format_args!(
					"duplicate reflect type: `{}`",
					registration.type_info().type_path(),
				)));
			}

			let value = map.next_value_seed(TypedValueDeserializer {
				registration,
				registry: self.registry,
			})?;
			values.push(value);
		}

		Ok(values)
	}
}

/// Deserializes one component or resource value of a known registered type,
/// converting through [`ReflectFromReflect`] so the value carries its concrete
/// type rather than staying a dynamic. The one reader of a value slot, shared
/// by the file's component map and a scene document's per-component apply.
pub(super) struct TypedValueDeserializer<'a> {
	/// The registration of the type the value must be.
	pub registration: &'a TypeRegistration,
	/// The registry the type's fields resolve through.
	pub registry: &'a TypeRegistry,
}

impl<'a, 'de> DeserializeSeed<'de> for TypedValueDeserializer<'a> {
	type Value = Box<dyn PartialReflect>;

	fn deserialize<D>(
		self,
		deserializer: D,
	) -> core::result::Result<Self::Value, D::Error>
	where
		D: Deserializer<'de>,
	{
		let value =
			TypedReflectDeserializer::new(self.registration, self.registry)
				.deserialize(deserializer)?;
		// attempt to convert using FromReflect to retain the concrete type.
		self.registration
			.data::<ReflectFromReflect>()
			.and_then(|from_reflect| {
				from_reflect.from_reflect(value.as_partial_reflect())
			})
			.map(PartialReflect::into_partial_reflect)
			.unwrap_or(value)
			.xok()
	}
}

#[cfg(test)]
mod test {
	use super::DynamicTemplateDeserializer;
	use super::DynamicTemplateSerializer;
	use crate::prelude::*;
	use serde::Deserialize;
	use serde::Serialize;
	use serde::de::DeserializeSeed;

	// de/serialize as hex, to exercise the custom serde path.
	mod qux {
		use crate::prelude::*;
		use serde::Deserializer;
		use serde::Serializer;
		use serde::de::Error;

		pub fn serialize<S>(
			value: &u32,
			serializer: S,
		) -> Result<S::Ok, S::Error>
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

	#[derive(
		Component,
		Copy,
		Clone,
		Reflect,
		Debug,
		PartialEq,
		Serialize,
		Deserialize,
	)]
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
			registry
				.register_type_data::<String, bevy_reflect::ReflectSerialize>();
			registry.register::<[usize; 3]>();
			registry.register::<(f32, f32)>();
		}
		world.insert_resource(registry);
		world
	}

	/// Serialize then deserialize the template via RON.
	#[cfg(feature = "ron")]
	fn roundtrip_ron(world: &World) -> DynamicTemplate {
		let template = {
			let registry = world.resource::<AppTypeRegistry>().read();
			TemplateBuilder::from_world(world, &registry)
				.extract_entities(
					world.iter_entities().map(|entity| entity.id()),
				)
				.build()
		};
		let registry = world.resource::<AppTypeRegistry>().read();
		let serialized = ron::ser::to_string(&DynamicTemplateSerializer::new(
			&template, &registry,
		))
		.unwrap();
		DynamicTemplateDeserializer {
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
		world.spawn_template(deserialized).unwrap();
		world
			.query::<&Qux>()
			.single(&world)
			.unwrap()
			.xpect_eq(Qux(42));
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
		let template = TemplateBuilder::from_world(&world, &registry)
			.extract_entities(world.iter_entities().map(|entity| entity.id()))
			.build();
		let serialized = postcard::to_allocvec(
			&DynamicTemplateSerializer::new(&template, &registry),
		)
		.unwrap();

		let deserialized = DynamicTemplateDeserializer {
			type_registry: &registry,
		}
		.deserialize(&mut postcard::Deserializer::from_bytes(&serialized))
		.unwrap();

		deserialized.entities.len().xpect_eq(1);
		let value = match &deserialized.entities[0].components[0] {
			ComponentSlot::Value(value) => value,
			ComponentSlot::Template(_) => panic!("expected a value slot"),
		};
		value
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
