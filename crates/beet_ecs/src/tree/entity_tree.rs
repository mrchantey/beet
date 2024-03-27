use crate::prelude::*;
use bevy::prelude::*;


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

	// pub fn dynamic_tree(&self, world: &World) -> Result<Tree<DynamicEntity>> {
	// 	self.0
	// 		.map(|e| DynamicEntity::new(world, *e))
	// 		.collect::<Result<_>>()
	// }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	fn component_tree() -> Result<()> {
		let mut world = World::new();
		let target = world.spawn_empty().id();
		let tree = test_constant_behavior_tree().spawn(&mut world, target);
		expect(tree.children.len()).to_be(2)?;

		let entity = tree.children[1].value;
		world.entity_mut(entity).insert(Score::Pass);
		let scores = tree.component_tree::<Score>(&world);

		expect(scores.value).to_be(Some(&Score::Fail))?;
		expect(scores.children[0].value).to_be(Some(&Score::Fail))?;
		expect(scores.children[1].value).to_be(Some(&Score::Pass))?;
		expect(scores.children[1].children[0].value)
			.to_be(Some(&Score::Fail))?;

		Ok(())
	}
}
