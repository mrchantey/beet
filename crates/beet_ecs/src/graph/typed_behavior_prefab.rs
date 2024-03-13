use super::type_registry_utils::append_beet_type_registry_with_generics;
use crate::prelude::*;
use anyhow::Result;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::World;
use bevy_reflect::FromReflect;
use bevy_reflect::Reflect;
use bevy_scene::serde::SceneDeserializer;
use bevy_scene::DynamicScene;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
use std::ops::Deref;
use std::ops::DerefMut;

/// This the 'instantiated' version of a [`BeetNode`].
/// It is a wrapper around a [`DynamicScene`] containing the behavior graph.
/// It implements [`Serialize`] and [`Deserialize`]
pub struct TypedBehaviorPrefab<T: ActionTypes> {
	pub prefab: BehaviorPrefab,
	_phantom: std::marker::PhantomData<T>,
}

impl<T: ActionTypes> Deref for TypedBehaviorPrefab<T> {
	type Target = BehaviorPrefab;
	fn deref(&self) -> &Self::Target { &self.prefab }
}

impl<T: ActionTypes> DerefMut for TypedBehaviorPrefab<T> {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.prefab }
}

/// Attempts a clone of this prefab
/// # Panics
/// if [`DynamicScene::write_to_world`] errors
impl<T: ActionTypes> Clone for TypedBehaviorPrefab<T> {
	fn clone(&self) -> Self {
		let mut tmp_world = World::new();
		let mut entity_map = EntityHashMap::default();
		self.scene
			.write_to_world(&mut tmp_world, &mut entity_map)
			.unwrap();
		let scene = DynamicScene::from_world(&tmp_world);
		let root = *entity_map.get(&self.root).unwrap();
		Self::new(scene, root)
	}
}

impl<T: ActionTypes> TypedBehaviorPrefab<T> {
	pub fn new(scene: DynamicScene, root: Entity) -> Self {
		let registry = Self::type_registry();
		Self {
			prefab: BehaviorPrefab::new(scene, root, registry),
			_phantom: std::marker::PhantomData,
		}
	}

	pub fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		append_beet_type_registry_with_generics::<T>(&registry);
		registry
	}

	// / This method will insert the corresponding AppTypeRegistry, or append it if it already exists
	pub fn from_world(mut src_world: World, root: Entity) -> Self {
		src_world.init_resource::<AppTypeRegistry>();
		let registry = src_world.resource_mut::<AppTypeRegistry>();
		append_beet_type_registry_with_generics::<T>(&registry);
		let scene = DynamicScene::from_world(&src_world);

		Self::new(scene, root)
	}
	pub fn into_world(&self) -> Result<World> {
		let mut dst_world = World::new();
		let registry = Self::type_registry();
		dst_world.insert_resource(registry);
		self.spawn(&mut dst_world, None)?;
		Ok(dst_world)
	}
}


impl<T: ActionTypes> Serialize for TypedBehaviorPrefab<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.prefab.serialize(serializer)
	}
}


impl<'de, T: ActionTypes> Deserialize<'de> for TypedBehaviorPrefab<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let registry = Self::type_registry();

		let scene_deserializer = SceneDeserializer {
			type_registry: &registry.read(),
		};

		let scene = scene_deserializer.deserialize(deserializer)?;

		let root = scene
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
						.map(|_root| dyn_entity.entity)
					})
					.next()
			})
			.next()
			.ok_or(serde::de::Error::custom(
				"Failed to deserialize behavior graph, no root entity",
			))?;

		Ok(Self::new(scene, root))
	}
}
