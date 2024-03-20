use anyhow::Result;
use beet::prelude::*;
use bevy::prelude::*;
use bevy::scene::DynamicEntity;

/// Wrapper for [`DynamicEntity`] that is [`Clone`] and [`PartialEq`]
#[derive(Clone, PartialEq)]
pub struct DynNode {
	pub entity: Entity,
	pub components: Vec<DynComponent>,
}

impl DynNode {
	pub fn new(world: &World, entity: Entity) -> Result<Self> {
		let dyn_entity = DynamicEntity::new(world, entity)?;
		let this = Self::new_from_dynamic(&dyn_entity);
		Ok(this)
	}
	pub fn new_from_dynamic(other: &DynamicEntity) -> Self {
		Self {
			entity: other.entity,
			components: other
				.components
				.iter()
				.map(|c| DynComponent::new(c.as_ref()))
				.collect(),
		}
	}

	pub fn name(&self) -> String {
		self.components
			.iter()
			.find_map(|c| c.get::<NodeName>())
			.map(|n| n.0)
			.unwrap_or_else(|| "New Node".to_string())
	}

	pub fn get<T: FromReflect>(&self) -> Option<T> {
		self.components.iter().find_map(|c| c.get::<T>())
	}
	pub fn set<T: Reflect + TypePath>(&mut self, value: &T) {
		if let Some(comp) =
			self.components.iter_mut().find(|c| c.represents::<T>())
		{
			comp.set(value)
		} else {
			self.components.push(DynComponent::new(value));
		}
	}

	pub fn remove<T: Reflect + TypePath>(&mut self) {
		self.components.retain(|c| false == c.represents::<T>());
	}

	pub fn children(&self) -> Vec<Entity> {
		self.components
			.iter()
			.find_map(|c| c.get::<Edges>())
			.map(|n| n.0)
			.unwrap_or_default()
	}

	pub fn add_child(&mut self, child: Entity) {
		if let Some(mut edges) = self.get::<Edges>() {
			edges.push(child);
			self.set(&edges);
		} else {
			self.set(&Edges(vec![child]));
		}
	}

	pub fn remove_child(&mut self, child: Entity) {
		if let Some(mut edges) = self.get::<Edges>() {
			edges.retain(|e| *e != child);
			if edges.len() == 0 {
				self.remove::<Edges>();
			} else {
				self.set(&edges);
			}
		}
	}

	pub fn new_tree(tree: &Tree<DynamicEntity>) -> Tree<DynNode> {
		tree.map(|e| DynNode::new_from_dynamic(e))
	}

	pub fn apply(self, world: &mut World) -> Result<()> {
		self.into_dynamic().apply(world)
	}

	pub fn into_dynamic(self) -> DynamicEntity {
		DynamicEntity {
			entity: self.entity,
			components: self.components.into_iter().map(|c| c.take()).collect(),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[derive(Debug, PartialEq, Component, Reflect)]
	struct MyStruct(pub i32);

	#[test]
	fn test_dyn_node() -> Result<()> {
		// Create a world and an entity
		let mut world = World::new();
		let registry = AppTypeRegistry::default();
		registry.write().register::<MyStruct>();
		registry.write().register::<NodeName>();
		registry.write().register::<Edges>();
		world.insert_resource(registry);

		let entity = world.spawn(MyStruct(2)).id();

		// Create a DynNode from the entity
		let mut dyn_node = DynNode::new(&world, entity)?;

		expect(dyn_node.name().as_str()).to_be("New Node")?;
		dyn_node.set(&NodeName::new("Foobar"));
		expect(dyn_node.name().as_str()).to_be("Foobar")?;

		expect(dyn_node.get::<Edges>()).to_be_none()?;
		expect(dyn_node.children().len()).to_be(0)?;


		let child = world.spawn_empty().id();
		dyn_node.add_child(child);
		expect(dyn_node.get::<Edges>()).to_be_some()?;
		expect(dyn_node.children().len()).to_be(1)?;


		dyn_node.remove_child(child);

		expect(dyn_node.get::<Edges>()).to_be_none()?;
		expect(dyn_node.children().len()).to_be(0)?;

		let dynamic_entity = dyn_node.into_dynamic();
		expect(dynamic_entity.entity).to_be(entity)?;
		expect(dynamic_entity.components.len()).to_be(1)?;
		Ok(())
	}
}
