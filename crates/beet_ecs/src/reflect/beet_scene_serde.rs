use crate::prelude::*;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::scene::serde::SceneDeserializer;
use bevy::scene::serde::SceneSerializer;
use bevy::scene::DynamicScene;
use serde::de::DeserializeSeed;
use serde::Deserialize;
use serde::Serialize;
use std::marker::PhantomData;

/// Basic serde functionality for a scene
pub struct BeetSceneSerde<T: ActionTypes> {
	pub scene: DynamicScene,
	phantom: PhantomData<T>,
}

impl<T: ActionTypes> BeetSceneSerde<T> {
	pub fn new(scene: DynamicScene) -> Self {
		Self {
			scene,
			phantom: PhantomData,
		}
	}

	pub fn type_registry() -> AppTypeRegistry {
		let registry = AppTypeRegistry::default();
		append_beet_type_registry_with_generics::<T>(&registry);
		registry
	}
}

impl<T: ActionTypes> Serialize for BeetSceneSerde<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let registry = Self::type_registry();
		let scene_serializer = SceneSerializer::new(&self.scene, &registry);
		scene_serializer.serialize(serializer)
	}
}

impl<'de, T: ActionTypes> Deserialize<'de> for BeetSceneSerde<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let registry = Self::type_registry();
		let scene_deserializer = SceneDeserializer {
			type_registry: &registry.read(),
		};
		let scene = scene_deserializer.deserialize(deserializer)?;

		Ok(Self {
			scene,
			phantom: PhantomData,
		})
	}
}
