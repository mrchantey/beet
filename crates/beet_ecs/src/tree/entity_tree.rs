use crate::prelude::*;
use bevy::prelude::*;
use std::fmt;

/// A tree of entities, useful for tests and debugging.
#[derive(Debug, Clone, Deref, DerefMut, Component)]
pub struct EntityTree(pub Tree<Entity>);

impl fmt::Display for EntityTree {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl EntityTree {
	pub fn new(entity: Entity) -> Self { Self(Tree::new(entity)) }
	pub fn new_with_children(
		entity: Entity,
		children: Vec<Tree<Entity>>,
	) -> Self {
		Self(Tree::new_with_children(entity, children))
	}

	pub fn new_with_world(entity: Entity, world: &World) -> Self {
		let mut tree = Tree::new(entity);
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children {
				tree = tree.with_child(Self::new_with_world(*child, world).0);
			}
		}
		Self(tree)
	}

	/// Get a tree of `Option<&T>` from the [`EntityTree`].
	pub fn component_tree<'a, T: Component>(
		&self,
		world: &'a World,
	) -> Tree<Option<&'a T>> {
		self.0.map(|e| world.get::<T>(*e))
	}
	pub fn ident(&self) -> EntityIdent { EntityIdent::new(self.0.value) }
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
		let tree = test_constant_behavior_tree(&mut world);
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
