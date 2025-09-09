use crate::prelude::*;
use bevy::prelude::*;


/// A tree where each node is an [`Entity`].
pub type EntityTree = TreeNode<Entity>;


impl EntityTree {
	/// Create a new [`EntityTree`] from a root [`Entity`],
	/// mapping its children.
	pub fn new_with_world(entity: Entity, world: &World) -> Self {
		let mut tree = TreeNode::new(entity);
		if let Some(children) = world.entity(entity).get::<Children>() {
			for child in children {
				tree = tree.with_child(Self::new_with_world(*child, world));
			}
		}
		tree
	}

	/// Get a tree of `Option<&T>` from the [`EntityTree`].
	pub fn component_tree<'a, T: Component>(
		&self,
		world: &'a World,
	) -> TreeNode<Option<&'a T>> {
		self.map(|e| world.get::<T>(*e))
	}
	// #[cfg(feature = "reflect")]
	// pub fn ident(&self) -> EntityIdent { EntityIdent::new(self.0.value) }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;


	fn create_tree(world: &mut World) -> EntityTree {
		let entity = world
			.spawn(Name::new("parent"))
			.with_children(|parent| {
				parent.spawn(Name::new("child1"));
				parent.spawn(Name::new("child2")).with_children(|parent| {
					parent.spawn(Name::new("grandchild1"));
				});
			})
			.id();
		EntityTree::new_with_world(entity, world)
	}


	#[test]
	fn component_tree() {
		let mut world = World::new();
		let tree = create_tree(&mut world);
		tree.children.len().xpect_eq(2);

		let entity = tree.children[1].value;
		world.entity_mut(entity).insert(Name::new("child2new"));
		let scores = tree.component_tree::<Name>(&world);

		scores.value.xpect_eq(Some(&Name::new("parent")));
		scores.children[0]
			.value
			.xpect_eq(Some(&Name::new("child1")));
		scores.children[1]
			.value
			.xpect_eq(Some(&Name::new("child2new")));
		scores.children[1].children[0]
			.value
			.xpect_eq(Some(&Name::new("grandchild1")));
	}
}
