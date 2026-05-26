//! Scene serialization and deserialization utilities.
//!
//! Provides [`SceneSaver`] and [`SceneLoader`] for converting world state
//! to and from various formats.

use crate::prelude::*;
use bevy::asset::AssetPath;
use bevy::asset::AssetServer;
use bevy::asset::LoadFromPath;
use bevy::asset::UntypedHandle;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::world_serialization::DynamicWorld;
use bevy::world_serialization::DynamicWorldBuilder;
use bevy::world_serialization::WorldFilter;
use bevy::world_serialization::serde::DynamicWorldSerializer;
use bevy::world_serialization::serde::WorldDeserializer;
use core::any::TypeId;

/// Serializes world state or a subtree to various formats.
///
/// Use [`SceneSaver::new`] for the full world, or [`SceneSaver::new`] followed
/// by [`SceneSaver::with_entity_tree`] to serialize only an entity and its descendants.
///
/// Extraction is deferred until [`SceneSaver::save`], as the underlying
/// [`DynamicWorldBuilder`] borrows the [`AppTypeRegistry`] for its lifetime.
pub struct SceneSaver<'a> {
	world: &'a World,
	component_filter: WorldFilter,
	resource_filter: WorldFilter,
	entities: Vec<Entity>,
	extract_resources: bool,
}

impl<'a> SceneSaver<'a> {
	/// Creates a saver for the entire world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			component_filter: WorldFilter::default(),
			resource_filter: WorldFilter::default(),
			entities: Vec::new(),
			extract_resources: false,
		}
	}

	/// Creates a saver that extracts all entities and resources, denying [`Time<Real>`].
	///
	/// Equivalent to the old `world.build_scene()` behavior.
	pub fn new_default(world: &'a mut World) -> Self {
		let all_entities: Vec<Entity> =
			world.query::<Entity>().iter(world).collect();
		Self::new(world)
			.with_entities(all_entities)
			.deny_resource::<Time<Real>>()
			.extract_resources()
	}

	/// Scopes serialization to an entity and its descendants.
	pub fn with_entity_tree(mut self, entity: Entity) -> Self {
		let mut entities = Vec::new();
		self.collect_descendants(entity, &mut entities);
		self.entities.extend(entities);
		self
	}

	/// Scopes serialization to a specific set of entities.
	pub fn with_entities(
		mut self,
		entities: impl IntoIterator<Item = Entity>,
	) -> Self {
		self.entities.extend(entities);
		self
	}

	/// Extracts all resources into the scene.
	pub fn extract_resources(mut self) -> Self {
		self.extract_resources = true;
		self
	}

	/// Denies a resource type from being included in the scene.
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.resource_filter = self.resource_filter.deny::<T>();
		self
	}

	/// Denies a component type from being included in the scene.
	pub fn deny_component<T: Component>(mut self) -> Self {
		self.component_filter = self.component_filter.deny::<T>();
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
		let registry = self.world.resource::<AppTypeRegistry>();
		let registry = registry.read();
		let mut builder =
			DynamicWorldBuilder::from_world(self.world, &registry)
				.with_component_filter(self.component_filter)
				.with_resource_filter(self.resource_filter)
				.extract_entities(self.entities.into_iter());
		if self.extract_resources {
			builder = builder.extract_resources();
		}
		let dyn_world = builder.build();
		let serializer = DynamicWorldSerializer::new(&dyn_world, &registry);
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

/// A no-op [`LoadFromPath`] used when deserializing scenes in a world that has
/// no [`AssetServer`]. Beet scenes do not currently serialize asset handles, so
/// this should never be invoked; if it is, an [`AssetServer`] is required.
struct NoAssetLoader;

impl LoadFromPath for NoAssetLoader {
	fn load_from_path_erased(
		&mut self,
		_type_id: TypeId,
		path: AssetPath<'static>,
	) -> UntypedHandle {
		panic!(
			"cannot deserialize asset handle for {path:?}: \
			 the world has no AssetServer"
		)
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
		use serde::de::DeserializeSeed;
		// `AssetServer` is cloned (cheap arc) out so the deserializer can hold a
		// `LoadFromPath` without borrowing the world for the read lock. Beet
		// scenes don't currently serialize asset handles, so a no-op loader
		// suffices when no server is present.
		let mut loader: Box<dyn LoadFromPath> =
			match self.world.get_resource::<AssetServer>() {
				Some(server) => Box::new(server.clone()),
				None => Box::new(NoAssetLoader),
			};
		let type_registry = self.world.resource::<AppTypeRegistry>().clone();
		let registry = type_registry.read();
		let dynamic_world = match bytes.media_type() {
			MediaType::Ron => {
				let text = bytes.as_utf8()?;
				let mut de = ron::de::Deserializer::from_str(text)?;
				WorldDeserializer {
					type_registry: &registry,
					load_from_path: &mut *loader,
				}
				.deserialize(&mut de)?
			}
			MediaType::Json => {
				cfg_if! {
					if #[cfg(feature = "json")] {
						let mut de =
							serde_json::Deserializer::from_slice(bytes.bytes());
						WorldDeserializer {
							type_registry: &registry,
							load_from_path: &mut *loader,
						}
						.deserialize(&mut de)?
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
						let mut de =
							postcard::Deserializer::from_bytes(bytes.bytes());
						WorldDeserializer {
							type_registry: &registry,
							load_from_path: &mut *loader,
						}
						.deserialize(&mut de)?
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
		};
		drop(registry);
		self.write(dynamic_world)
	}

	fn write(self, dynamic_world: DynamicWorld) -> Result<Vec<Entity>> {
		let entity = self.entity;
		let mut default_map = EntityHashMap::default();
		let entity_map = self.entity_map.unwrap_or(&mut default_map);
		dynamic_world.write_to_world(self.world, entity_map)?;

		let spawned: Vec<Entity> = entity_map.values().copied().collect();
		if let Some(parent) = entity {
			for entity in spawned.iter() {
				self.world.entity_mut(*entity).insert(SceneOf(parent));
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
#[relationship(relationship_target = SceneEntities)]
pub struct SceneOf(pub Entity);

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
#[relationship_target(relationship = SceneOf,linked_spawn)]
pub struct SceneEntities(Vec<Entity>);



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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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

	#[crate::test]
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

		// Spawned entities should have SceneOf pointing to target
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<SceneOf>()
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

	#[crate::test]
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

		// Spawned entities have SceneOf, not ChildOf
		spawned.len().xpect_eq(1);
		app.world()
			.entity(spawned[0])
			.get::<SceneOf>()
			.unwrap()
			.0
			.xpect_eq(target);
	}
}
