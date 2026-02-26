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
/// by [`SceneSaver::for_entity`] to serialize only an entity and its descendants.
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

	/// Serializes to a RON string.
	pub fn save_ron(self) -> Result<String> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		let pretty_config = ron::ser::PrettyConfig::default()
			.indentor("  ".to_string())
			.new_line("\n".to_string());
		ron::ser::to_string_pretty(&serializer, pretty_config)?.xok()
	}

	/// Serializes to a JSON string.
	#[cfg(feature = "json")]
	pub fn save_json(self) -> Result<String> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		serde_json::to_string_pretty(&serializer)?.xok()
	}

	/// Serializes to postcard bytes.
	#[cfg(feature = "postcard")]
	pub fn save_postcard(self) -> Result<Vec<u8>> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		postcard::to_allocvec(&serializer)?.xok()
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
pub struct SceneLoader<'a> {
	world: &'a mut World,
	entity_map: Option<&'a mut EntityHashMap<Entity>>,
}

impl<'a> SceneLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self {
		Self {
			world,
			entity_map: None,
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

	/// Deserializes a RON scene string into the world.
	pub fn load_ron(self, scene: impl AsRef<str>) -> Result {
		use serde::de::DeserializeSeed;
		let mut de = ron::de::Deserializer::from_str(scene.as_ref())?;
		let dynamic_scene = {
			let type_registry = self.world.resource::<AppTypeRegistry>();
			let scene_de = bevy::scene::serde::SceneDeserializer {
				type_registry: &type_registry.read(),
			};
			scene_de.deserialize(&mut de)?
		};
		self.write(dynamic_scene)
	}

	/// Deserializes a JSON scene string into the world.
	#[cfg(feature = "json")]
	pub fn load_json(self, scene: impl AsRef<str>) -> Result {
		use serde::de::DeserializeSeed;
		let mut de = serde_json::Deserializer::from_str(scene.as_ref());
		let dynamic_scene = {
			let type_registry = self.world.resource::<AppTypeRegistry>();
			let scene_de = bevy::scene::serde::SceneDeserializer {
				type_registry: &type_registry.read(),
			};
			scene_de.deserialize(&mut de)?
		};
		self.write(dynamic_scene)
	}

	/// Deserializes postcard bytes into the world.
	#[cfg(feature = "postcard")]
	pub fn load_postcard(self, bytes: &[u8]) -> Result {
		use serde::de::DeserializeSeed;
		let mut de = postcard::Deserializer::from_bytes(bytes);
		let dynamic_scene = {
			let type_registry = self.world.resource::<AppTypeRegistry>();
			let registry_read = type_registry.read();
			let scene_de = bevy::scene::serde::SceneDeserializer {
				type_registry: &registry_read,
			};
			scene_de.deserialize(&mut de)?
		};
		self.write(dynamic_scene)
	}

	fn write(self, dynamic_scene: bevy::scene::DynamicScene) -> Result {
		let mut default_map = EntityHashMap::default();
		let entity_map = self.entity_map.unwrap_or(&mut default_map);
		dynamic_scene.write_to_world(self.world, entity_map)?;
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	fn scene_world() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.init();
		app.update();
		app
	}

	#[test]
	fn round_trip_ron() {
		let mut app = scene_world();
		let scene =
			SceneSaver::new_default(app.world_mut()).save_ron().unwrap();
		scene.xref().xpect_contains("Time");
		SceneLoader::new(app.world_mut()).load_ron(&scene).unwrap();
	}

	#[test]
	fn entity_scope() {
		let mut app = scene_world();
		let entity = app.world_mut().spawn(Name::new("Root")).id();
		app.world_mut()
			.entity_mut(entity)
			.with_child(Name::new("Child"));

		let scene = SceneSaver::new(app.world_mut())
			.with_entity_tree(entity)
			.save_ron()
			.unwrap();
		scene.xref().xpect_contains("Root");
		scene.xref().xpect_contains("Child");
	}

	#[test]
	fn custom_entity_map() {
		let mut app = scene_world();
		let scene =
			SceneSaver::new_default(app.world_mut()).save_ron().unwrap();
		let mut entity_map = Default::default();
		SceneLoader::new(app.world_mut())
			.with_entity_map(&mut entity_map)
			.load_ron(&scene)
			.unwrap();
	}
}
