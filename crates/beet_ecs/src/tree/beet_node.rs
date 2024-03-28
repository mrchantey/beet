use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::entity::EntityHashMap;
use bevy::prelude::*;
use std::any::TypeId;


#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deref, DerefMut)]
pub struct BeetNode(pub Entity);

impl BeetNode {
	pub fn new(entity: Entity) -> Self { Self(entity) }

	pub fn get_roots(world: &mut World) -> Vec<BeetNode> {
		world
			.query_filtered::<Entity, With<BeetRoot>>()
			.iter(world)
			.map(|e| BeetNode::new(e))
			.collect()
	}

	/// SrcWorld will not be mutated, but querystate doesnt have non-mutable option
	pub fn deep_clone(self, world: &mut World) -> Result<Self> {
		let entities = Edges::collect_world(*self, world);
		let scene = DynamicSceneBuilder::from_world(world)
			.extract_entities(entities.into_iter())
			.build();

		let mut entity_map = EntityHashMap::default();
		scene.write_to_world(world, &mut entity_map)?;

		Ok(Self::new(entity_map[&*self]))
	}
	pub fn deep_clone_to_dest(
		self,
		src_world: &mut World,
		dst_world: &mut World,
	) -> Result<Self> {
		let entities = Edges::collect_world(*self, src_world);
		let scene = DynamicSceneBuilder::from_world(src_world)
			.extract_entities(entities.into_iter())
			.build();

		let mut entity_map = EntityHashMap::default();
		scene.write_to_world(dst_world, &mut entity_map)?;

		Ok(Self::new(entity_map[&*self]))
	}

	pub fn bind_agent(self, world: &mut World, agent: Entity) {
		world.entity_mut(agent).insert(AgentMarker);
		Edges::visit_dfs_world(world, *self, |world, entity| {
			world.entity_mut(entity).insert(TargetAgent(agent));
		});
	}

	/// Add a node as a child of the given entity
	pub fn add_child(self, world: &mut World) -> BeetNode {
		let mut entity = world.spawn_empty();
		BeetBuilder::insert_default_components(
			&mut entity,
			"New Node".to_string(),
		);
		let entity = entity.id();

		if let Some(mut parent) = world.get_entity_mut(*self) {
			if let Some(mut edges) = parent.get_mut::<Edges>() {
				edges.push(entity);
			} else {
				parent.insert(Edges(vec![entity]));
			}
		} else {
			log::warn!("parent not found when adding node");
		}

		BeetNode(entity)
	}

	pub fn children(self, world: &World) -> Vec<BeetNode> {
		world
			.get::<Edges>(*self)
			.map(|edges| {
				edges.0.iter().map(|entity| BeetNode(*entity)).collect()
			})
			.unwrap_or_default()
	}
	pub fn add_default_component(
		self,
		world: &mut World,
		component: TypeId,
	) -> Result<()> {
		ComponentIdent::new(*self, component).add(world)
	}


	pub fn remove(self, world: &mut World) {
		// 1. remove children recursive
		for child in self.children(world) {
			child.remove(world);
		}

		// 2. despawn
		world.despawn(*self);

		// 3. remove from parent lists
		for mut edges in world.query::<&mut Edges>().iter_mut(world) {
			edges.retain(|e| *e != *self);
		}
	}


	pub fn components(self, world: &World) -> Vec<ComponentIdent> {
		ComponentUtils::get(world, *self)
			.into_iter()
			.map(|c| ComponentIdent::new(*self, c))
			.collect()
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

	fn node(world: &mut World) -> BeetNode {
		BeetBuilder::new(SetOnRun(RunResult::Success))
			.spawn_no_target(world)
			.node()
	}

	#[test]
	fn deep_clone() -> Result<()> {
		let mut world = world();
		let node1 = node(&mut world);

		expect(world.entities().len()).to_be(1)?;

		let node2 = node1.deep_clone(&mut world)?;

		expect(world.entities().len()).to_be(2)?;

		expect(&world)
			.component(*node2)?
			.to_be(&SetOnRun(RunResult::Success))?;

		expect(BeetNode::get_roots(&mut world).len()).to_be(2)?;

		Ok(())
	}
	#[test]
	fn bind() -> Result<()> {
		let mut world = world();
		let node = node(&mut world);

		let agent = world.spawn_empty().id();

		node.bind_agent(&mut world, agent);

		expect(&world).component(agent)?.to_be(&AgentMarker)?;

		expect(&world)
			.component(*node)?
			.to_be(&TargetAgent(agent))?;

		Ok(())
	}
	#[test]
	fn children() -> Result<()> {
		let mut world = World::new();
		let node = test_no_action_behavior_tree()
			.spawn_no_target(&mut world)
			.node();

		expect(node.children(&world).len()).to_be(2)?;
		let child = node.add_child(&mut world);
		expect(node.children(&world).len()).to_be(3)?;
		child.remove(&mut world);
		expect(node.children(&world).len()).to_be(2)?;

		Ok(())
	}
	#[test]
	fn components() -> Result<()> {
		let mut world = World::new();
		let node = test_no_action_behavior_tree()
			.spawn_no_target(&mut world)
			.node();

		expect(node.components(&world).len()).to_be(7)?;

		Ok(())
	}
}
