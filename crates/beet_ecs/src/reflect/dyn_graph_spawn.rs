use super::dyn_graph::DynGraph;
use crate::prelude::AgentMarker;
use crate::prelude::TargetAgent;
use anyhow::Result;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;

impl DynGraph {
	pub fn spawn(
		&self,
		dst_world: &mut World,
		target_agent: Entity,
	) -> Result<Entity> {
		let entity_map = self.spawn_no_target(dst_world)?;
		let src_root = self.root();
		dst_world.entity_mut(target_agent).insert(AgentMarker);
		for (_src, dst) in entity_map.iter() {
			dst_world.entity_mut(*dst).insert(TargetAgent(target_agent));
		}
		let dst_root = entity_map.get(&src_root).ok_or_else(|| {
			anyhow::anyhow!("Root entity not found in entity map")
		})?;
		Ok(*dst_root)
	}

	pub fn spawn_no_target(
		&self,
		dst_world: &mut World,
	) -> Result<EntityHashMap<Entity>> {
		let src_world = self.world().read();
		let scene = DynamicScene::from_world(&src_world);
		let mut entity_map = EntityHashMap::default();
		scene.write_to_world(dst_world, &mut entity_map)?;
		Ok(entity_map)
	}
}

