use super::serde::WorldDeserializer;
use crate::prelude::*;
use bevy::ecs::entity::EntityHashMap;

/// Deserializes world state from various formats.
///
/// An optional [`EntityHashMap`] can be provided via [`WorldSerdeLoader::with_entity_map`]
/// to remap entity identifiers on load. If none is provided, a default map is used.
///
/// If an entity is provided via [`WorldSerdeLoader::with_entity`], all spawned root entities
/// (those without a [`ChildOf`] relationship) are tracked as [`WorldSerdeEntities`] of that entity.
pub struct WorldSerdeLoader<'a> {
	world: &'a mut World,
	entity_map: Option<&'a mut EntityHashMap<Entity>>,
	/// If set, all spawned entities are tracked as [`WorldSerdeEntities`] of this entity.
	entity: Option<Entity>,
}

impl<'a> WorldSerdeLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			entity_map: None,
			entity: None,
		}
	}

	/// Creates a loader for the world containing the given entity.
	pub fn new_entity(entity: EntityWorldMut<'a>) -> Self {
		let id = entity.id();
		Self {
			world: entity.into_world_mut(),
			entity_map: None,
			entity: Some(id),
		}
	}

	/// Provides a custom entity map to use during loading.
	pub fn with_entity_map(
		mut self,
		entity_map: &'a mut EntityHashMap<Entity>,
	) -> Self {
		self.entity_map = Some(entity_map);
		self
	}

	/// Tracks all spawned entities as [`WorldSerdeEntities`] of the given entity.
	pub fn with_entity(mut self, entity: Entity) -> Self {
		self.entity = Some(entity);
		self
	}

	/// Deserializes from [`MediaBytes`] into the world, dispatching by media type.
	pub fn load(self, bytes: &MediaBytes) -> Result<Vec<Entity>> {
		use serde::de::DeserializeSeed;
		let type_registry = self.world.resource::<AppTypeRegistry>().clone();
		let registry = type_registry.read();
		let dynamic_world = match bytes.media_type() {
			MediaType::Ron => {
				let text = bytes.as_utf8()?;
				let mut de = ron::de::Deserializer::from_str(text)?;
				WorldDeserializer {
					type_registry: &registry,
				}
				.deserialize(&mut de)?
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						let mut de =
							serde_json::Deserializer::from_slice(bytes.bytes());
						WorldDeserializer { type_registry: &registry }
							.deserialize(&mut de)?
					} else {
						bevybail!("The `json` feature is required for JSON loading")
					}
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				cfg_if! {
					if #[cfg(feature = "postcard")] {
						let mut de =
							postcard::Deserializer::from_bytes(bytes.bytes());
						WorldDeserializer { type_registry: &registry }
							.deserialize(&mut de)?
					} else {
						bevybail!("The `postcard` feature is required for postcard loading")
					}
				}
			}
			other => {
				bevybail!("Unsupported media type for world serde loading: {other}")
			}
		};
		drop(registry);
		self.write(dynamic_world)
	}

	fn write(self, dynamic_world: super::DynamicWorld) -> Result<Vec<Entity>> {
		let entity = self.entity;
		let mut default_map = EntityHashMap::default();
		let entity_map = self.entity_map.unwrap_or(&mut default_map);
		dynamic_world.write_to_world(self.world, entity_map)?;

		let spawned: Vec<Entity> = entity_map.values().copied().collect();
		if let Some(parent) = entity {
			for entity in spawned.iter() {
				self.world.entity_mut(*entity).insert(WorldSerdeOf(parent));
			}
		}

		Ok(spawned)
	}
}

/// Added to entities spawned by the loader to track their source entity in the
/// serialized data.
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship(relationship_target = WorldSerdeEntities)]
pub struct WorldSerdeOf(pub Entity);

/// Tracks the entities spawned into a target via [`WorldSerdeLoader::with_entity`].
#[derive(
	Debug,
	Clone,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
#[relationship_target(relationship = WorldSerdeOf, linked_spawn)]
pub struct WorldSerdeEntities(Vec<Entity>);

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn serde_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.register_type::<Name>();
		app.init();
		app.update();
		app
	}

	#[crate::test]
	fn round_trip_ron() {
		let mut app = serde_world();
		let world_serde_bytes = WorldSerdeSaver::new_default(app.world_mut())
			.save(MediaType::Ron)
			.unwrap();
		world_serde_bytes.as_utf8().unwrap().xref().xpect_contains("Time");
		WorldSerdeLoader::new(app.world_mut())
			.load(&world_serde_bytes)
			.unwrap();
	}

	#[crate::test]
	fn entity_scope() {
		let mut app = serde_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut().entity_mut(entity).with_child(Name::new("Child"));

		let world_serde_bytes = WorldSerdeSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save(MediaType::Ron)
			.unwrap();
		let text = world_serde_bytes.as_utf8().unwrap();
		text.xref().xpect_contains("Root");
		text.xref().xpect_contains("Child");
	}

	#[crate::test]
	fn custom_entity_map() {
		let mut app = serde_world();
		let world_serde_bytes = WorldSerdeSaver::new_default(app.world_mut())
			.save(MediaType::Ron)
			.unwrap();
		let mut entity_map = Default::default();
		WorldSerdeLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load(&world_serde_bytes)
			.unwrap();
	}

	#[crate::test]
	fn loads_into_entity_adds_world_serde_of() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("WorldSerdeChild")).id();
		let world_serde_bytes = WorldSerdeSaver::new(app.world_mut())
			.with_entities([child])
			.save(MediaType::Ron)
			.unwrap();

		let target = app.world_mut().spawn(Name::new("Target")).id();
		let spawned = WorldSerdeLoader::new(app.world_mut())
			.with_entity(target)
			.load(&world_serde_bytes)
			.unwrap();

		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<WorldSerdeOf>()
			.unwrap()
			.0
			.xpect_eq(target);
		app.world()
			.entity(spawned[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("WorldSerdeChild");
	}

	#[crate::test]
	fn loads_into_entity_preserves_existing_children() {
		let mut app = serde_world();
		let child = app.world_mut().spawn(Name::new("WorldSerdeChild")).id();
		let world_serde_bytes = WorldSerdeSaver::new(app.world_mut())
			.with_entities([child])
			.save(MediaType::Ron)
			.unwrap();

		let target = app
			.world_mut()
			.spawn((Name::new("Target"), children![Name::new("OldChild")]))
			.id();
		app.world()
			.entity(target)
			.get::<Children>()
			.unwrap()
			.len()
			.xpect_eq(1);

		let spawned = WorldSerdeLoader::new(app.world_mut())
			.with_entity(target)
			.load(&world_serde_bytes)
			.unwrap();

		// existing children are preserved
		let children: Vec<Entity> = app
			.world()
			.entity(target)
			.get::<Children>()
			.unwrap()
			.iter()
			.collect();
		children.len().xpect_eq(1);
		app.world()
			.entity(children[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("OldChild");

		// spawned entities have WorldSerdeOf, not ChildOf
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<WorldSerdeOf>()
			.unwrap()
			.0
			.xpect_eq(target);
	}
}
