use crate::prelude::*;
use bevy::prelude::*;
use bevy::scene::DynamicEntity;


#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct EntityTree(pub Tree<Entity>);

impl EntityTree {
	pub fn new(entity: Entity) -> Self { Self(Tree::new(entity)) }
	pub fn new_with_children(
		entity: Entity,
		children: Vec<Tree<Entity>>,
	) -> Self {
		Self(Tree::new_with_children(entity, children))
	}
	pub fn bind_agent(&self, world: &mut World, agent: Entity) {
		world.entity_mut(agent).insert(AgentMarker);
		Self::bind_agent_inner(&self.0, agent, world);
	}

	fn bind_agent_inner(tree: &Tree<Entity>, agent: Entity, world: &mut World) {
		world.entity_mut(tree.value).insert(TargetAgent(agent));
		for child in tree.children.iter() {
			Self::bind_agent_inner(child, agent, world);
		}
	}

	pub fn visit_dfs(&self, visitor: &mut dyn FnMut(Entity)) {
		Self::visit_dfs_inner(&self.0, visitor);
	}

	fn visit_dfs_inner(tree: &Tree<Entity>, visitor: &mut dyn FnMut(Entity)) {
		visitor(tree.value);
		for child in tree.children.iter() {
			Self::visit_dfs_inner(child, visitor);
		}
	}

	pub fn component_tree<'a, T: Component>(
		&self,
		world: &'a World,
	) -> Tree<Option<&'a T>> {
		self.0.map(|e| world.get::<T>(*e))
	}

	pub fn dynamic_tree(&self, world: &World) -> Tree<DynamicEntity> {
		self.0.map(|e| DynamicEntity::new(world, *e))
	}
}
