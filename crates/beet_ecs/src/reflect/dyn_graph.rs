use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct DynGraph {
	world: Arc<RwLock<World>>,
	root: Entity,
}

impl DynGraph {
	pub fn world(&self) -> &Arc<RwLock<World>> { &self.world }
	pub fn nodes(&self) -> Vec<Entity> {
		self.world.read().iter_entities().map(|e| e.id()).collect()
	}

	pub fn new<T: ActionList>(node: BeetNode) -> Self {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		append_beet_type_registry_with_generics::<T>(&mut world.resource());

		let root = node.spawn_no_target(&mut world).value;

		Self {
			world: Arc::new(RwLock::new(world)),
			root,
		}
	}

	pub fn root(&self) -> Entity { self.root }

	pub fn type_registry(&self) -> AppTypeRegistry {
		self.world.read().resource::<AppTypeRegistry>().clone()
	}

	pub fn get_node(&self, entity: Entity) -> DynNode {
		let world = self.world.read();
		DynNode::new(&world, entity)
	}
	pub fn set_node(&self, node: DynNode) -> Result<()> {
		let mut world = self.world.write();
		node.apply(&mut world)?;
		Ok(())
	}

	pub fn add_node(&self, parent: Entity) -> Entity {
		let mut world = self.world.write();
		let mut entity = world.spawn_empty();
		BeetNode::insert_default_components(&mut entity, 0);
		let entity = entity.id();

		if let Some(mut parent) = world.get_entity_mut(parent) {
			if let Some(mut edges) = parent.get_mut::<Edges>() {
				edges.push(entity);
			} else {
				parent.insert(Edges(vec![entity]));
			}
		} else {
			log::warn!("parent not found when adding node");
		}

		entity
	}

	pub fn children(&self, entity: Entity) -> Vec<Entity> {
		self.world
			.read()
			.get::<Edges>(entity)
			.map(|e| e.0.clone())
			.unwrap_or_default()
	}

	pub fn remove_node(&self, entity: Entity) {
		// 1. remove children recursive
		let world = self.world.read();
		let children = world
			.get::<Edges>(entity)
			.map(|e| e.0.clone())
			.unwrap_or_default();
		drop(world);
		for child in children {
			self.remove_node(child);
		}

		// 2. despawn
		let mut world = self.world.write();
		world.despawn(entity);

		// 3. remove from parent lists
		for mut edges in world.query::<&mut Edges>().iter_mut(&mut world) {
			edges.retain(|e| *e != entity);
		}
	}

	// pub fn into_dynamic_scene(&self) -> DynamicScene {
	// 	DynamicScene {
	// 		resources: Default::default(),
	// 		entities: self
	// 			.nodes
	// 			.values()
	// 			.map(|n| n.clone().into_dynamic_entity())
	// 			.collect(),
	// 	}
	// }
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[derive(Debug, PartialEq, Component, Reflect)]
	#[reflect(Component)]
	struct MyStruct(pub i32);

	#[test]
	fn add_remove_children() -> Result<()> {
		pretty_env_logger::try_init().ok();

		// Create a world and an entity
		let graph = (EmptyAction.child((EmptyAction, SetOnRun(Score::Pass))))
			.into_graph::<EcsNode>();

		expect(graph.nodes().len()).to_be(2)?;
		let root = graph.root();

		expect(graph.children(root).len()).to_be(1)?;
		let child = graph.add_node(root);
		expect(graph.children(root).len()).to_be(2)?;

		expect(graph.nodes().len()).to_be(3)?;

		graph.remove_node(child);
		expect(graph.nodes().len()).to_be(2)?;
		expect(graph.children(root).len()).to_be(1)?;


		Ok(())
	}
	#[test]
	fn edit_node() -> Result<()> {
		pretty_env_logger::try_init().ok();

		// Create a world and an entity
		let graph = (EmptyAction.child((EmptyAction, SetOnRun(Score::Pass))))
			.into_graph::<EcsNode>();

		graph.type_registry().write().register::<MyStruct>();

		expect(graph.nodes().len()).to_be(2)?;
		let mut node = graph.get_node(graph.root());
		node.set(&MyStruct(3));
		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.to_be_none()?;
		graph.set_node(node.clone())?;
		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.as_some()?
			.to_be(&MyStruct(3))?;

		node.remove::<MyStruct>();
		graph.set_node(node.clone())?;

		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.to_be_none()?;
		Ok(())
	}
}
