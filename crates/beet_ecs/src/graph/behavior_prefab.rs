use crate::prelude::*;
use anyhow::Result;
use bevy_core::Name;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::World;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_scene::serde::SceneDeserializer;
use bevy_scene::serde::SceneSerializer;
use bevy_scene::DynamicScene;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;

/// This the 'instantiated' version of a [`BeetNode`].
/// It is a wrapper around a [`DynamicScene`] containing the behavior graph.
/// It implements [`Serialize`] and [`Deserialize`]
pub struct BehaviorPrefab<T: ActionTypes> {
	pub scene: DynamicScene,
	// pub root: Entity,
	_phantom: std::marker::PhantomData<T>,
}

/// Attempts a clone of this prefab
/// # Panics
/// if [`DynamicScene::write_to_world`] errors
impl<T: ActionTypes> Clone for BehaviorPrefab<T> {
	fn clone(&self) -> Self {
		let mut tmp_world = World::new();
		let mut entity_map = EntityHashMap::default();
		self.scene
			.write_to_world(&mut tmp_world, &mut entity_map)
			.unwrap();
		let scene = DynamicScene::from_world(&tmp_world);
		Self::new(scene)
	}
}

impl<T: ActionTypes> BehaviorPrefab<T> {
	pub fn new(scene: DynamicScene) -> Self {
		Self {
			scene,
			_phantom: std::marker::PhantomData,
		}
	}

	// / This method will insert the corresponding AppTypeRegistry, or append it if it already exists
	pub fn from_world(mut src_world: World) -> Self {
		Self::append_type_registry_with_world(&mut src_world);
		let scene = DynamicScene::from_world(&src_world);

		Self::new(scene)
	}
	pub fn into_world(&self) -> Result<World> {
		let mut dst_world = World::new();
		let registry = Self::get_type_registry();
		dst_world.insert_resource(registry);
		self.spawn(&mut dst_world, None)?;
		Ok(dst_world)
	}

	/// Prefabs are [`DynamicScene`]s so can only be spawned using [`World`]. This means spawn systems must be exclusive.
	/// If the world doesn't have a type registry, one matching this prefab will be added.
	/// # Errors
	/// If the world's [`AppTypeRegistry`] is missing a type in the graph
	pub fn spawn(
		&self,
		dst_world: &mut impl IntoWorld,
		target: Option<Entity>,
	) -> Result<EntityGraph> {
		let dst_world = dst_world.into_world_mut();
		Self::append_type_registry_with_world(dst_world);

		let mut entity_map = EntityHashMap::default();
		self.scene.write_to_world(dst_world, &mut entity_map)?;

		// TODO we should track root through conversion, way easier and faster
		let root = entity_map
			.values()
			.filter(|entity| {
				dst_world.entity(**entity).contains::<BehaviorGraphRoot>()
			})
			.next()
			.ok_or(anyhow::anyhow!(
				"Failed to spawn behavior graph, no root entity"
			))?;

		if let Some(target) = target {
			dst_world.entity_mut(target).insert(AgentMarker);
			for entity in entity_map.values() {
				dst_world.entity_mut(*entity).insert(TargetAgent(target));
			}
		}

		let entity_graph = EntityGraph::from_world(dst_world, *root);
		Ok(entity_graph)
	}

	// pub fn root(&self) -> Entity { **self.world.resource::<SerdeRootEntity>() }
	pub fn get_type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		Self::append_type_registry(&registry);
		registry
	}

	pub fn append_type_registry_with_world(world: &mut World) {
		if let Some(mut registry) = world.get_resource_mut::<AppTypeRegistry>()
		{
			Self::append_type_registry(&mut registry);
		} else {
			world.insert_resource(Self::get_type_registry());
		}
	}

	/// Register all types in [`T`] as well as those that get appended
	/// to the graph by [`EntityGraph::spawn_with_options`]
	/// with the exception of [[`TargetAgent`]] which gets reattached via [`BehaviorPrefab::spawn`]
	pub fn append_type_registry(registry: &AppTypeRegistry) {
		let mut registry = registry.write();
		registry.register::<Name>();
		registry.register::<Edges>();
		registry.register::<Running>();
		registry.register::<RunTimer>();
		registry.register::<BehaviorGraphRoot>();
		T::register(&mut registry);
	}
}


impl<T: ActionTypes> Serialize for BehaviorPrefab<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let registry = Self::get_type_registry();
		let scene_serializer = SceneSerializer::new(&self.scene, &registry);
		scene_serializer.serialize(serializer)
	}
}


impl<'de, T: ActionTypes> Deserialize<'de> for BehaviorPrefab<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let registry = Self::get_type_registry();

		let scene_deserializer = SceneDeserializer {
			type_registry: &registry.read(),
		};

		let scene = scene_deserializer.deserialize(deserializer)?;

		let _root = scene
			.entities
			.iter()
			.filter_map(|dyn_entity| {
				dyn_entity
					.components
					.iter()
					.filter_map(|dyn_component| {
						let cloned: Box<dyn Reflect> =
							dyn_component.clone_value();
						<BehaviorGraphRoot as FromReflect>::from_reflect(
							&*cloned,
						)
					})
					.next()
			})
			.next()
			.ok_or(serde::de::Error::custom(
				"Failed to deserialize behavior graph, no root entity",
			))?;

		Ok(Self::new(scene))
	}
}

pub trait IntoBehaviorPrefab<M> {
	fn into_prefab<T: ActionTypes>(self) -> Result<BehaviorPrefab<T>>;
}
