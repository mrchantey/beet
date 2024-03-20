use crate::prelude::*;
use anyhow::Result;
use bevy::prelude::*;
use parking_lot::RwLock;
use std::any::TypeId;
use std::sync::Arc;

#[derive(Clone)]
pub struct DynGraph {
	world: Arc<RwLock<World>>,
	root: Entity,
	component_types: Vec<ComponentType>,
}

impl DynGraph {
	pub fn new<T: ActionList>(node: BeetNode) -> Self {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		T::register_components(&mut world);
		append_beet_type_registry_with_generics::<T>(&mut world.resource());

		let root = node.spawn_no_target(&mut world).value;

		Self {
			component_types: ComponentType::from_world(&world),
			world: Arc::new(RwLock::new(world)),
			root,
		}
	}
	pub fn world(&self) -> &Arc<RwLock<World>> { &self.world }
	pub fn component_types(&self) -> Vec<ComponentType> {
		self.component_types.clone()
	}
	pub fn nodes(&self) -> Vec<Entity> {
		self.world.read().iter_entities().map(|e| e.id()).collect()
	}

	pub fn type_registry(&self) -> AppTypeRegistry {
		self.world.read().resource::<AppTypeRegistry>().clone()
	}
	pub fn root(&self) -> Entity { self.root }

	pub fn children(&self, entity: Entity) -> Vec<Entity> {
		self.world
			.read()
			.get::<Edges>(entity)
			.map(|e| e.0.clone())
			.unwrap_or_default()
	}
	pub fn get_node(&self, entity: Entity) -> Result<DynNode> {
		let world = self.world.read();
		DynNode::new(&world, entity)
	}


	pub fn set_node(&self, node: DynNode) -> Result<()> {
		let mut world = self.world.write();
		node.apply(&mut world)?;
		Ok(())
	}

	/// Add a node as a child of the given entity
	pub fn add_node(&self, parent: Entity) -> Entity {
		let mut world = self.world.write();
		let mut entity = world.spawn_empty();
		BeetNode::insert_default_components(
			&mut entity,
			"New Node".to_string(),
		);
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

	pub fn remove_node(&self, entity: Entity) {
		// 1. remove children recursive
		for child in self.children(entity) {
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


	pub fn set_component<T: Component>(
		&self,
		entity: Entity,
		component: T,
	) -> Result<()> {
		let mut world = self.world.write();
		world
			.get_entity_mut(entity)
			.map(|mut e| {
				e.insert(component);
			})
			.ok_or_else(|| anyhow::anyhow!("entity not found"))
	}
	pub fn get_component<T: Component + Clone>(
		&self,
		entity: Entity,
	) -> Option<T> {
		let world = self.world.read();
		world
			.get_entity(entity)
			.map(|e| e.get::<T>())
			.flatten()
			.map(|c| c.clone())
	}

	pub fn add_component(&self, entity: Entity, type_id: TypeId) -> Result<()> {
		let registry = self.type_registry();
		let registry = registry.read();
		let registration = registry
			.get(type_id)
			.ok_or_else(|| anyhow::anyhow!("type not found: {:?}", type_id))?;
		let reflect_default =
			registration.data::<ReflectDefault>().ok_or_else(|| {
				anyhow::anyhow!("type is not ReflectDefault, try adding #[reflect(Default)]")
			})?;

		let mut node = self.get_node(entity)?;
		let new_value: Box<dyn Reflect> = reflect_default.default();
		node.components.push(DynComponent::new(new_value.as_ref()));
		self.set_node(node)?;
		Ok(())
	}
	// this is awkward but we dont have a `world.entity().remove_by_id yet`
	pub fn remove_component(
		&self,
		entity: Entity,
		type_id: TypeId,
	) -> Result<()> {
		let mut node = self.get_node(entity)?;
		let before = node.components.len();
		node.components
			.retain(|c| c.represented_type_info().type_id() != type_id);
		if before == node.components.len() {
			anyhow::bail!("component not found");
		}

		self.set_node(node)?;
		Ok(())
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
	use std::any::TypeId;
	use sweet::*;

	#[derive(Debug, Default, PartialEq, Component, Reflect)]
	#[reflect(Default, Component)]
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
		let mut node = graph.get_node(graph.root())?;
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
	#[test]
	fn edit_components() -> Result<()> {
		// setup
		pretty_env_logger::try_init().ok();
		let graph = (EmptyAction.child((EmptyAction, SetOnRun(Score::Pass))))
			.into_graph::<EcsNode>();
		graph.type_registry().write().register::<MyStruct>();

		let mut node = graph.get_node(graph.root())?;

		// add normally
		node.set(&MyStruct(3));
		graph.set_node(node.clone())?;
		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.to_be_some()?;
		// remove
		graph.remove_component(node.entity, TypeId::of::<MyStruct>())?;
		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.to_be_none()?;
		// add as default
		graph.add_component(node.entity, TypeId::of::<MyStruct>())?;
		expect(graph.world().read().get::<MyStruct>(graph.root()))
			.to_be_some()?;

		Ok(())
	}
}
