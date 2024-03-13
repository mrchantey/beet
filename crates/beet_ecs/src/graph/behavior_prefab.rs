use super::type_registry_utils::append_beet_type_registry;
use super::type_registry_utils::merge_type_registries;
use crate::prelude::*;
use anyhow::Result;
use bevy_ecs::entity::Entity;
use bevy_ecs::entity::EntityHashMap;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::world::World;
use bevy_scene::serde::SceneSerializer;
use bevy_scene::DynamicScene;
use serde::Serialize;


pub struct BehaviorPrefab {
	pub scene: DynamicScene,
	pub root: Entity,
	pub registry: AppTypeRegistry,
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

		if let Some(target) = target {
			dst_world.entity_mut(target).insert(AgentMarker);
			for entity in entity_map.values() {
				dst_world.entity_mut(*entity).insert(TargetAgent(target));
			}
		}

		let entity_graph = EntityGraph::from_world(dst_world, *root);
		Ok(entity_graph)
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
