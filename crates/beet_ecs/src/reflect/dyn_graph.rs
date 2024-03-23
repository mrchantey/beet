use crate::prelude::*;
use anyhow::Result;
use bevy::ecs::component::ComponentId;
use bevy::prelude::*;
use bevy::ptr::OwningPtr;
use bevy::reflect::ReflectFromPtr;
use parking_lot::RwLock;
use std::any::TypeId;
use std::ptr::NonNull;
use std::sync::Arc;

/// Wrapper around a world in which every entity is a node on a behavior graph
#[derive(Clone, Resource)]
pub struct DynGraph {
	world: Arc<RwLock<World>>,
	root: Entity,
}

impl DynGraph {
	pub fn new<T: ActionList>(node: BeetNode) -> Self {
		let mut world = World::new();
		world.init_resource::<AppTypeRegistry>();
		T::register_components(&mut world);
		append_beet_type_registry_with_generics::<T>(&mut world.resource());

		let root = node.spawn_no_target(&mut world).value;

		Self {
			world: Arc::new(RwLock::new(world)),
			root,
		}
	}
	pub fn new_with(world: Arc<RwLock<World>>, root: Entity) -> Self {
		Self { world, root }
	}
	pub fn into_serde<T: ActionTypes>(&self) -> DynGraphSerde<T> { self.into() }

	pub fn world(&self) -> &Arc<RwLock<World>> { &self.world }
	pub fn component_types(&self) -> Vec<ComponentType> {
		ComponentType::from_world(&self.world.read())
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
	/// This is expensive, only use when you need to
	pub fn get_node(&self, entity: Entity) -> Result<DynNode> {
		let world = self.world.read();
		DynNode::new(&world, entity)
	}

	/// This is expensive, only use when you need to
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

	/// gets the type id of components that are valid rust types.
	pub fn get_components(&self, entity: Entity) -> Vec<TypeId> {
		let world = self.world.read();
		if world.get_entity(entity).is_none() {
			Default::default()
		} else {
			world
				.inspect_entity(entity)
				.into_iter()
				.filter_map(|c| c.type_id())
				.collect()
		}
	}


	pub fn component_id(&self, type_id: TypeId) -> Result<ComponentId> {
		self.world
			.read()
			.components()
			.get_id(type_id)
			.ok_or_else(|| {
				anyhow::anyhow!("component not registered: {type_id:?}")
			})
	}


	pub fn set_component_typed<T: Component>(
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
	pub fn get_component_typed<T: Component + Clone>(
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
		let new_value: Box<dyn Reflect> = reflect_default.default();

		let component_id = self.component_id(type_id)?;
		let mut world = self.world.write();
		if let Some(mut entity) = world.get_entity_mut(entity) {
			unsafe {
				let non_null =
					NonNull::new_unchecked(Box::into_raw(new_value) as *mut _);
				let ptr = OwningPtr::new(non_null);
				entity.insert_by_id(component_id, ptr);
			}
			Ok(())
		} else {
			anyhow::bail!("entity not found: {entity:?}")
		}
		// let mut node = self.get_node(entity)?;
		// node.components.push(DynComponent::new(new_value.as_ref()));
		// self.set_node(node)?;
	}
	/// Expensive and awkward but we dont have a `world.entity().remove_by_id yet`
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

	fn map_component<O>(
		&self,
		entity: Entity,
		type_id: TypeId,
		func: impl FnOnce(&dyn Reflect) -> O,
	) -> Result<O> {
		let registry = self.type_registry();
		let registry = registry.read();
		let Some(registration) = registry.get(type_id) else {
			anyhow::bail!("type not registered: {type_id:?}")
		};
		let component_id = self.component_id(type_id)?;
		let world = self.world.read();
		let Some(entity) = world.get_entity(entity) else {
			anyhow::bail!("entity not found: {entity:?}")
		};
		let Some(component) = entity.get_by_id(component_id) else {
			anyhow::bail!("component not in entity: {type_id:?}")
		};
		let value = unsafe {
			registration
				.data::<ReflectFromPtr>()
				.unwrap()
				.as_reflect(component)
		};
		Ok(func(value))
	}
	fn map_component_mut<O>(
		&self,
		entity: Entity,
		type_id: TypeId,
		func: impl FnOnce(&mut dyn Reflect) -> O,
	) -> Result<O> {
		let registry = self.type_registry();
		let registry = registry.read();
		let Some(registration) = registry.get(type_id) else {
			anyhow::bail!("type not registered: {type_id:?}")
		};
		// drop(registry);
		let component_id = self.component_id(type_id)?;
		let mut world = self.world.write();
		let Some(mut entity) = world.get_entity_mut(entity) else {
			anyhow::bail!("entity not found: {entity:?}")
		};
		let Some(component) = entity.get_mut_by_id(component_id) else {
			anyhow::bail!("component not in entity: {type_id:?}")
		};
		// component.
		let value = unsafe {
			registration
				.data::<ReflectFromPtr>()
				.unwrap()
				.as_reflect_mut(component.into_inner())
		};
		Ok(func(value))
	}
	pub fn get_component(
		&self,
		entity: Entity,
		type_id: TypeId,
	) -> Result<Box<dyn Reflect>> {
		self.map_component(entity, type_id, |c| c.clone_value())
	}

	pub fn map_field<'p, O>(
		&self,
		entity: Entity,
		component: TypeId,
		path: impl ReflectPath<'p>,
		func: impl Fn(&dyn Reflect) -> O,
	) -> Result<O> {
		self.map_component(entity, component, |component| {
			let field = component
				.reflect_path(path)
				.map_err(|e| anyhow::anyhow!("{e}"))?;
			Ok(func(field))
		})?
	}
	pub fn map_field_mut<'p, O>(
		&self,
		entity: Entity,
		component: TypeId,
		path: impl ReflectPath<'p>,
		func: impl Fn(&mut dyn Reflect) -> O,
	) -> Result<O> {
		self.map_component_mut(entity, component, |component| {
			let field = component
				.reflect_path_mut(path)
				.map_err(|e| anyhow::anyhow!("{e}"))?;
			Ok(func(field))
		})?
	}
	pub fn get_field<'p>(
		&self,
		entity: Entity,
		component: TypeId,
		path: impl ReflectPath<'p>,
	) -> Result<Box<dyn Reflect>> {
		let component = self.get_component(entity, component)?;
		let val = component
			.reflect_path(path)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		Ok(val.clone_value())
	}

	pub fn set_field<'p>(
		&self,
		entity: Entity,
		component: TypeId,
		path: impl ReflectPath<'p>,
		new_value: &dyn Reflect,
	) -> Result<()> {
		self.map_field_mut(entity, component, path, move |field| {
			field.apply(new_value);
			Ok(())
		})?
	}
	pub fn set_component(
		&self,
		entity: Entity,
		component: TypeId,
		new_value: &dyn Reflect,
	) -> Result<()> {
		self.map_component_mut(entity, component, move |current| {
			current.apply(new_value);
			Ok(())
		})?
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use bevy::reflect::Access;
	use bevy::reflect::ParsedPath;
	use std::any::TypeId;
	use sweet::*;

	#[derive(Debug, Default, PartialEq, Component, Reflect)]
	#[reflect(Default, Component)]
	struct MyStruct(pub i32);

	fn default_graph() -> DynGraph {
		BeetNode::new(EmptyAction).into_graph::<EcsNode>()
	}

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
		let graph = default_graph();

		graph.type_registry().write().register::<MyStruct>();

		expect(graph.nodes().len()).to_be(1)?;
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
		let graph = default_graph();
		graph.type_registry().write().register::<MyStruct>();

		let entity = graph.root();

		expect(graph.get_components(entity).len()).to_be_greater_than(0)?;

		let mut node = graph.get_node(entity)?;

		// add normally
		node.set(&MyStruct(3));
		graph.set_node(node.clone())?;
		expect(graph.world().read().get::<MyStruct>(entity)).to_be_some()?;
		// remove
		graph.remove_component(node.entity, TypeId::of::<MyStruct>())?;
		expect(graph.world().read().get::<MyStruct>(entity)).to_be_none()?;
		// add as default
		graph.add_component(node.entity, TypeId::of::<MyStruct>())?;
		expect(graph.world().read().get::<MyStruct>(entity)).to_be_some()?;

		Ok(())
	}
	#[test]
	fn edit_fields() -> Result<()> {
		// setup
		pretty_env_logger::try_init().ok();
		let graph = default_graph();
		graph.world().write().init_component::<MyStruct>();
		graph.type_registry().write().register::<MyStruct>();

		let entity = graph.root();
		let type_id = TypeId::of::<MyStruct>();
		graph.add_component(entity, type_id)?;

		//set componnet
		graph.set_component(entity, type_id, &MyStruct(1))?;
		expect(
			graph
				.get_component(entity, type_id)?
				.reflect_partial_eq(&MyStruct(1))
				.unwrap_or_default(),
		)
		.to_be_true()?;


		//set root value
		let path: Vec<Access> = vec![];
		let path = ParsedPath::from(path);
		graph.set_field(entity, type_id, &path, &MyStruct(2))?;

		expect(
			graph
				.get_field(entity, type_id, &path)?
				.reflect_partial_eq(&MyStruct(2))
				.unwrap_or_default(),
		)
		.to_be_true()?;

		//set nested value
		let path = ParsedPath::from(vec![Access::TupleIndex(0)]);
		graph.set_field(entity, type_id, &path, &3)?;

		expect(
			graph
				.get_field(entity, type_id, &path)?
				.reflect_partial_eq(&3)
				.unwrap_or_default(),
		)
		.to_be_true()?;

		Ok(())
	}
}
