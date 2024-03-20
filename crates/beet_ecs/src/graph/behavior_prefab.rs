use super::type_registry_utils::append_beet_type_registry;
use super::type_registry_utils::merge_type_registries;
use super::typed_behavior_prefab::TypedBehaviorPrefab;
use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use bevy::scene::serde::SceneSerializer;
use serde::Serialize;
use std::fmt;


pub struct BehaviorPrefab {
	pub scene: DynamicScene,
	pub root: Entity,
	pub registry: AppTypeRegistry,
}

impl fmt::Debug for BehaviorPrefab {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let scene = self.scene.serialize_ron(&self.registry).unwrap();
		f.debug_struct("BehaviorPrefab")
			.field("scene", &scene)
			.field("root", &self.root)
			.field("registry", &"TODO")
			.finish()
	}
}


// TODO this sucks, dyncomponent does this much better
/// Attempts a clone of this prefab
/// # Panics
/// if [`DynamicScene::write_to_world`] errors
impl Clone for BehaviorPrefab {
	fn clone(&self) -> Self {
		let mut tmp_world = World::new();
		let mut entity_map = EntityHashMap::default();
		self.scene
			.write_to_world(&mut tmp_world, &mut entity_map)
			.unwrap();
		let scene = DynamicScene::from_world(&tmp_world);
		let root = *entity_map.get(&self.root).unwrap();

		let mut new_registry = AppTypeRegistry::default();
		merge_type_registries(&self.registry, &mut new_registry);

		Self::new(scene, root, new_registry)
	}
}


impl BehaviorPrefab {
	pub fn new(
		scene: DynamicScene,
		root: Entity,
		registry: AppTypeRegistry,
	) -> Self {
		Self {
			scene,
			root,
			registry,
		}
	}

	/// This will append beet types to the world registry
	pub fn from_world(world: &mut World, root: Entity) -> Self {
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource::<AppTypeRegistry>().clone();
		append_beet_type_registry(&registry);

		let scene = DynamicScene::from_world(world);
		Self {
			scene,
			root,
			registry,
		}
	}

	/// Used for symmetry when passing a serializable into a struct that can also be deserialized
	pub fn typed<T: ActionTypes>(self) -> TypedBehaviorPrefab<T> {
		TypedBehaviorPrefab::from_prefab(self)
	}

	/// Prefabs are [`DynamicScene`]s so can only be spawned using [`World`]. This means spawn systems must be exclusive.
	/// If the world doesn't have a type registry, one matching this prefab will be added.
	/// # Errors
	/// If the world's [`AppTypeRegistry`] is missing a type in the graph
	pub fn spawn(&self, dst_world: &mut impl IntoWorld) -> Result<EntityTree> {
		let dst_world = dst_world.into_world_mut();
		dst_world.init_resource::<AppTypeRegistry>();
		let mut dst_registry = dst_world.resource_mut::<AppTypeRegistry>();

		merge_type_registries(&self.registry, dst_registry.as_mut());

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
		let graph = EntityGraph::from_world(dst_world, *root);
		let tree = graph.0.into_tree();
		Ok(EntityTree(tree))
	}
}

impl Serialize for BehaviorPrefab {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let scene_serializer =
			SceneSerializer::new(&self.scene, &self.registry);
		scene_serializer.serialize(serializer)
	}
}


pub trait IntoBehaviorPrefab<M> {
	fn into_prefab(self) -> Result<BehaviorPrefab>;
}
