use crate::prelude::*;
use bevy::prelude::*;

impl EntityIdent {

	pub fn get_roots(world: &mut World) -> Vec<EntityIdent> {
		world
			.query_filtered::<Entity, With<BeetRoot>>()
			.iter(world)
			.map(|e| EntityIdent::new(e))
			.collect()
	}


	pub fn bind_agent(self, world: &mut World, agent: Entity) {
		ChildrenExt::visit_dfs_world(world, *self, |world, entity| {
			world.entity_mut(entity).insert(TargetAgent(agent));
		});
	}

	/// Add a node as a child of the given entity
	pub fn add_child_behavior(self, world: &mut World) -> EntityIdent {
		let mut entity = world.spawn_empty();
		BeetBuilder::insert_default_components(
			&mut entity,
			"New Node".to_string(),
		);
		let entity = entity.id();

		if let Some(mut parent) = world.get_entity_mut(*self) {
			parent.add_child(entity);
		} else {
			log::warn!("parent not found when adding node");
		}

		EntityIdent(entity)
	}

	pub fn graph_roles(
		self,
		world: &World,
	) -> Vec<(ComponentIdent, GraphRole)> {
		self.components(world)
			.into_iter()
			.filter_map(|c| c.graph_role(world).ok().map(|role| (c, role)))
			.collect()
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;


	fn world() -> World {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		let registry = world.resource_mut::<AppTypeRegistry>();
		let mut registry = registry.write();
		registry.register::<SetOnRun<RunResult>>();
		registry.register::<BeetRoot>();
		drop(registry);
		world
	}

	fn node(world: &mut World) -> EntityIdent {
		BeetBuilder::new(SetOnRun(RunResult::Success))
			.spawn_no_target(world)
			.node()
	}

	#[test]
	fn bind() -> Result<()> {
		let mut world = world();
		let node = node(&mut world);

		let agent = world.spawn_empty().id();

		node.bind_agent(&mut world, agent);

		expect(&world)
			.component(*node)?
			.to_be(&TargetAgent(agent))?;

		Ok(())
	}

}
