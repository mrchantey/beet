//! Scene serialization and deserialization utilities.
//!
//! Provides [`SceneSaver`] and [`SceneLoader`] for converting world state
//! to and from various formats.

use crate::prelude::*;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use bevy::scene::serde::SceneSerializer;

/// Serializes world state or a subtree to various formats.
///
/// Use [`SceneSaver::new`] for the full world, or [`SceneSaver::new`] followed
/// by [`SceneSaver::with_entity_tree`] to serialize only an entity and its descendants.
pub struct SceneSaver<'a> {
	registry: TypeRegistryArc,
	world: &'a World,
	builder: DynamicSceneBuilder<'a>,
}

impl<'a> SceneSaver<'a> {
	/// Creates a saver for the entire world.
	pub fn new(world: &'a mut World) -> Self {
		let builder = DynamicSceneBuilder::from_world(world);
		let registry = world.resource::<AppTypeRegistry>().0.clone();
		Self {
			registry,
			world,
			builder,
		}
	}

	/// Creates a saver that extracts all entities and resources, denying [`Time<Real>`].
	///
	/// Equivalent to the old `world.build_scene()` behavior.
	pub fn new_default(world: &'a mut World) -> Self {
		let all_entities: Vec<Entity> =
			world.query::<Entity>().iter(world).collect();
		let mut saver = Self::new(world);
		saver.builder = saver
			.builder
			.extract_entities(all_entities.into_iter())
			.deny_resource::<Time<Real>>()
			.extract_resources();
		saver
	}

	/// Scopes serialization to an entity and its descendants.
	pub fn with_entity_tree(mut self, entity: Entity) -> Self {
		let mut entities = Vec::new();
		self.collect_descendants(entity, &mut entities);
		self.builder = self.builder.extract_entities(entities.into_iter());
		self
	}

	/// Scopes serialization to a specific set of entities.
	pub fn with_entities(
		mut self,
		entities: impl IntoIterator<Item = Entity>,
	) -> Self {
		self.builder = self.builder.extract_entities(entities.into_iter());
		self
	}

	/// Extracts all resources into the scene.
	pub fn extract_resources(mut self) -> Self {
		self.builder = self.builder.extract_resources();
		self
	}

	/// Denies a resource type from being included in the scene.
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.builder = self.builder.deny_resource::<T>();
		self
	}

	/// Denies a component type from being included in the scene.
	pub fn deny_component<T: Component>(mut self) -> Self {
		self.builder = self.builder.deny_component::<T>();
		self
	}

	/// Serializes to [`MediaBytes`] using the given format with default options.
	pub fn save(self, media_type: MediaType) -> Result<MediaBytes> {
		self.save_with_options(media_type, default())
	}

	/// Serializes to [`MediaBytes`] using the given format and [`SerializeOptions`].
	pub fn save_with_options(
		self,
		media_type: MediaType,
		options: SerializeOptions,
	) -> Result<MediaBytes> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		MediaBytes::serialize_with_options(media_type, &serializer, options)
	}

	/// Collects an entity and all its descendants into a flat list.
	fn collect_descendants(&self, entity: Entity, entities: &mut Vec<Entity>) {
		entities.push(entity);
		if let Some(children) = self.world.entity(entity).get::<Children>() {
			for child in children.iter() {
				self.collect_descendants(child, entities);
			}
		}
	}
}

/// Deserializes world state from various formats.
///
/// An optional [`EntityHashMap`] can be provided via [`SceneLoader::with_entity_map`]
/// to remap entity identifiers on load. If none is provided, a default map is used.
///
/// If an entity is provided via [`SceneLoader::with_entity`], all spawned root
/// entities (those without a [`ChildOf`] relationship) will be reparented as
/// children of that entity.
pub struct SceneLoader<'a> {
	world: &'a mut World,
	entity_map: Option<&'a mut EntityHashMap<Entity>>,
	/// If set, all spawned root entities are reparented as children of this entity.
	entity: Option<Entity>,
}

impl<'a> SceneLoader<'a> {
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

	/// Reparents all spawned root entities as children of the given entity.
	///
	/// Any existing children of the entity are removed before the scene
	/// roots are attached.
	pub fn with_entity(mut self, entity: Entity) -> Self {
		self.entity = Some(entity);
		self
	}

	/// Deserializes a scene from [`MediaBytes`] into the world,
	/// dispatching by media type.
	pub fn load(self, bytes: &MediaBytes) -> Result<Vec<Entity>> {
		match bytes.media_type() {
			MediaType::Ron => {
				use serde::de::DeserializeSeed;
				let text = bytes.as_utf8()?;
				let mut de = ron::de::Deserializer::from_str(text)?;
				let dynamic_scene = {
					let type_registry =
						self.world.resource::<AppTypeRegistry>();
					let scene_de = bevy::scene::serde::SceneDeserializer {
						type_registry: &type_registry.read(),
					};
					scene_de.deserialize(&mut de)?
				};
				self.write(dynamic_scene)
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						use serde::de::DeserializeSeed;
						let mut de =
							serde_json::Deserializer::from_slice(bytes.bytes());
						let dynamic_scene = {
							let type_registry =
								self.world.resource::<AppTypeRegistry>();
							let scene_de = bevy::scene::serde::SceneDeserializer {
								type_registry: &type_registry.read(),
							};
							scene_de.deserialize(&mut de)?
						};
						self.write(dynamic_scene)
					} else {
						bevybail!(
							"The `json` feature is required for JSON scene loading"
						)
					}
				}
			}
			MediaType::Postcard | MediaType::Bytes => {
				cfg_if! {
					if #[cfg(feature = "postcard")] {
						use serde::de::DeserializeSeed;
						let mut de =
							postcard::Deserializer::from_bytes(bytes.bytes());
						let dynamic_scene = {
							let type_registry =
								self.world.resource::<AppTypeRegistry>();
							let registry_read = type_registry.read();
							let scene_de = bevy::scene::serde::SceneDeserializer {
								type_registry: &registry_read,
							};
							scene_de.deserialize(&mut de)?
						};
						self.write(dynamic_scene)
					} else {
						bevybail!(
							"The `postcard` feature is required for postcard scene loading"
						)
					}
				}
			}
			other => {
				bevybail!("Unsupported media type for scene loading: {other}")
			}
		}
	}

	fn write(
		self,
		dynamic_scene: bevy::scene::DynamicScene,
	) -> Result<Vec<Entity>> {
		let entity = self.entity;
		let mut default_map = EntityHashMap::default();
		let entity_map = self.entity_map.unwrap_or(&mut default_map);
		dynamic_scene.write_to_world(self.world, entity_map)?;

		let spawned: Vec<Entity> = entity_map.values().copied().collect();
		if let Some(parent) = entity {
			for entity in spawned.iter() {
				self.world.entity_mut(*entity).insert(SpawnedBy(parent));
			}
		}

		Ok(spawned)
	}
}

/// Added to entities spawned by the scene loader to track their source entity in the scene file.
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
#[relationship(relationship_target = SpawnedEntities)]
pub struct SpawnedBy(pub Entity);

/// Added to the [`SceneLoader::Entity`]
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
#[relationship_target(relationship = SpawnedBy)]
pub struct SpawnedEntities(Vec<Entity>);



#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn scene_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.register_type::<Name>();
		app.init();
		app.update();
		app
	}

	#[test]
	fn round_trip_ron() {
		let mut app = scene_world();
		let scene_bytes = SceneSaver::new_default(app.world_mut())
			.save(MediaType::Ron)
			.unwrap();
		scene_bytes.as_utf8().unwrap().xref().xpect_contains("Time");
		SceneLoader::new(app.world_mut())
			.load(&scene_bytes)
			.unwrap();
	}

	#[test]
	fn entity_scope() {
		let mut app = scene_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut()
			.entity_mut(entity)
			.with_child(Name::new("Child"));

		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save(MediaType::Ron)
			.unwrap();
		let text = scene_bytes.as_utf8().unwrap();
		text.xref().xpect_contains("Root");
		text.xref().xpect_contains("Child");
	}

	#[test]
	fn custom_entity_map() {
		let mut app = scene_world();
		let scene_bytes = SceneSaver::new_default(app.world_mut())
			.save(MediaType::Ron)
			.unwrap();
		let mut entity_map = Default::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load(&scene_bytes)
			.unwrap();
	}

	#[test]
	fn loads_into_entity_adds_spawned_by() {
		let mut app = scene_world();
		// Spawn a named entity to form a scene
		let child = app.world_mut().spawn(Name::new("SceneChild")).id();
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([child])
			.save(MediaType::Ron)
			.unwrap();

		// Load the scene into a target entity
		let target = app.world_mut().spawn(Name::new("Target")).id();
		let spawned = SceneLoader::new(app.world_mut())
			.with_entity(target)
			.load(&scene_bytes)
			.unwrap();

		// Spawned entities should have SpawnedBy pointing to target
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<SpawnedBy>()
			.unwrap()
			.0
			.xpect_eq(target);
		app.world()
			.entity(spawned[0])
			.get::<Name>()
			.unwrap()
			.as_str()
			.xpect_eq("SceneChild");
	}

	#[test]
	fn loads_into_entity_preserves_existing_children() {
		let mut app = scene_world();
		let child = app.world_mut().spawn(Name::new("SceneChild")).id();
		let scene_bytes = SceneSaver::new(app.world_mut())
			.with_entities([child])
			.save(MediaType::Ron)
			.unwrap();

		// Spawn a target with an existing child
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

		let spawned = SceneLoader::new(app.world_mut())
			.with_entity(target)
			.load(&scene_bytes)
			.unwrap();

		// Existing children should be preserved
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

		// Spawned entities have SpawnedBy, not ChildOf
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<SpawnedBy>()
			.unwrap()
			.0
			.xpect_eq(target);
	}
}
