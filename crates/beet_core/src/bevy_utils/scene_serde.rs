//! Scene serialization and deserialization utilities.
//!
//! Provides [`SceneSaver`] and [`SceneLoader`] for converting world state
//! to and from various formats.

use crate::prelude::*;
use bevy::prelude::*;
use bevy::reflect::TypeRegistryArc;
use bevy::scene::serde::SceneSerializer;

/// Serializes world state or a subtree to various formats.
///
/// Use [`SceneSaver::new`] for the full world, or [`SceneSaver::new_for_entity`]
/// to serialize only an entity and its descendants.
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

	pub fn new_default(world: &'a mut World) -> Self {
		Self::new(world).deny_resource::<Time<Real>>()
	}

	/// Creates a saver scoped to an entity and its descendants.
	pub fn for_entity(mut self, entity: Entity) -> Self {
		let mut entities = Vec::new();
		self.collect_descendants(entity, &mut entities);
		self.builder = self.builder.extract_entities(entities.into_iter());
		self
	}

	pub fn extract_resources(mut self) -> Self {
		self.builder = self.builder.extract_resources();
		self
	}
	pub fn deny_resource<T: Resource>(mut self) -> Self {
		self.builder = self.builder.deny_resource::<T>();
		self
	}

	pub fn deny_component<T: Component>(mut self) -> Self {
		self.builder = self.builder.deny_component::<T>();
		self
	}

	/// Serializes to a RON string, denying [`bevy::time::TimeReal`] resources.
	///
	/// Uses the default builder that excludes [`Time<Real>`].
	pub fn save_ron(self) -> Result<String> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		let pretty_config = ron::ser::PrettyConfig::default()
			.indentor("  ".to_string())
			.new_line("\n".to_string());
		ron::ser::to_string_pretty(&serializer, pretty_config)?.xok()
	}

	/// Serializes to a JSON string, denying [`Time<Real>`] resources.
	#[cfg(feature = "json")]
	pub fn save_json(self) -> Result<String> {
		let registry = self.registry.read();
		let dyn_scene = self.builder.build();
		let serializer = SceneSerializer::new(&dyn_scene, &registry);
		serde_json::to_string_pretty(&serializer)?.xok()
	}

	/// Serializes to postcard bytes, denying [`Time<Real>`] resources.
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
pub struct SceneLoader<'a> {
	world: &'a mut World,
}

impl<'a> SceneLoader<'a> {
	/// Creates a loader for the given world.
	pub fn new(world: &'a mut World) -> Self { Self { world } }

	/// Deserializes a RON scene string into the world.
	pub fn load_ron(&mut self, scene: impl AsRef<str>) -> Result {
		self.load_ron_with(scene, &mut Default::default())
	}

	/// Deserializes a RON scene string into the world with a custom entity map.
	pub fn load_ron_with(
		&mut self,
		scene: impl AsRef<str>,
		entity_map: &mut bevy::ecs::entity::EntityHashMap<Entity>,
	) -> Result {
		self.load(
			&mut ron::de::Deserializer::from_str(scene.as_ref())?,
			entity_map,
		)
	}

	/// Deserializes a JSON scene string into the world.
	#[cfg(feature = "json")]
	pub fn load_json(&mut self, scene: impl AsRef<str>) -> Result {
		self.load(
			&mut serde_json::Deserializer::from_str(scene.as_ref()),
			&mut default(),
		)
	}

	/// Deserializes postcard bytes into the world.
	#[cfg(feature = "postcard")]
	pub fn load_postcard(&mut self, bytes: &[u8]) -> Result {
		self.load(
			&mut postcard::Deserializer::from_bytes(bytes),
			&mut default(),
		)
	}

	fn load<'de, D>(
		self,
		deserializer: &mut D,
		entity_map: &mut bevy::ecs::entity::EntityHashMap<Entity>,
	) -> Result
	where
		D: serde::Deserializer<'de>,
		D::Error: 'static + Send + Sync,
	{
		let dynamic_scene = {
			use serde::de::DeserializeSeed;
			let type_registry = self.world.resource::<AppTypeRegistry>();
			let scene_deserializer = bevy::scene::serde::SceneDeserializer {
				type_registry: &type_registry.read(),
			};
			scene_deserializer.deserialize(deserializer)?
		};
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
		let scene = SceneSaver::new(app.world_mut()).save_ron().unwrap();
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

		let scene = SceneSaver::new_for_entity(app.world_mut(), entity)
			.serialize_ron()
			.unwrap();
		scene.xref().xpect_contains("Root");
		scene.xref().xpect_contains("Child");
	}
}
