use crate::prelude::*;
use anyhow::Result;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::World;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_reflect::TypeRegistry;
use bevy_reflect::TypeRegistryArc;
use bevy_scene::serde::SceneDeserializer;
use bevy_scene::serde::SceneSerializer;
use bevy_scene::DynamicScene;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::RwLock;



/// This the 'instantiated' version of a [`BehaviorGraph`].
/// It is a wrapper around a [`DynamicScene`] containing the behavior graph.
/// It implements [`Serialize`] and [`Deserialize`]
pub struct BehaviorGraphPrefab<T: ActionTypes> {
	pub scene: DynamicScene,
	// pub root: Entity,
	_phantom: std::marker::PhantomData<T>,
}

impl<T: ActionTypes> BehaviorGraphPrefab<T> {
	pub fn new(scene: DynamicScene) -> Self {
		Self {
			scene,
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn from_graph<M>(graph: impl IntoBehaviorGraph<M>) -> Result<Self> {
		graph.into_behavior_graph().into_prefab()
	}

	/// # Panics
	/// If the world is missing one of the following:
	/// - [`SerdeRootEntity`]
	/// - [`AppTypeRegistry`]
	pub fn from_world(world: World) -> Self {
		let _registry = world.resource::<AppTypeRegistry>();
		let scene = DynamicScene::from_world(&world);
		Self::new(scene)
	}
	pub fn into_world(&self) -> Result<World> {
		let mut world = World::new();
		let registry = Self::get_type_registry();
		world.insert_resource(registry);
		self.spawn(&mut world, None)?;
		Ok(world)
	}

	/// If the world doesn't have a type registry, one matching this prefab will be added.
	/// # Errors
	/// If the world's [`AppTypeRegistry`] is missing a type in the graph
	pub fn spawn(
		&self,
		world: &mut World,
		target: Option<Entity>,
	) -> Result<Entity> {
		if false == world.contains_resource::<AppTypeRegistry>() {
			world.insert_resource(Self::get_type_registry());
		}

		let mut entity_map = EntityHashMap::default();
		self.scene.write_to_world(world, &mut entity_map)?;

		if let Some(target) = target {
			for entity in entity_map.values() {
				world.entity_mut(*entity).insert(TargetAgent(target));
			}
		}

		let root = entity_map
			.values()
			.filter(|entity| {
				world.entity(**entity).contains::<BehaviorGraphRoot>()
			})
			.next()
			.ok_or(anyhow::anyhow!(
				"Failed to spawn behavior graph, no root entity"
			))?;


		Ok(*root)
	}

	// pub fn root(&self) -> Entity { **self.world.resource::<SerdeRootEntity>() }
	pub fn get_type_registry() -> AppTypeRegistry {
		let mut registry = TypeRegistry::default();
		registry.register::<BehaviorGraphRoot>();
		T::register(&mut registry);
		let registry = AppTypeRegistry(TypeRegistryArc {
			internal: Arc::new(RwLock::new(registry)),
		});
		registry
	}
}


impl<T: ActionTypes> Serialize for BehaviorGraphPrefab<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let registry = Self::get_type_registry();
		let scene_serializer = SceneSerializer::new(&self.scene, &registry);
		scene_serializer.serialize(serializer)
	}
}


impl<'de, T: ActionTypes> Deserialize<'de> for BehaviorGraphPrefab<T> {
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
