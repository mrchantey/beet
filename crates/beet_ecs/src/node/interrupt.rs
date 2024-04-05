use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

/// Indicate this node should stop running.
/// As this is frequently added and removed, it is `SparseSet`.
#[derive(Default, Debug, Component, PartialEq)]
#[component(storage = "SparseSet")]
pub struct Interrupt;

pub fn sync_interrupts(
	mut commands: Commands,
	interrupts: Query<Entity, Added<Interrupt>>,
	children: Query<&Children>,
) {
	for entity in interrupts.iter() {
		ChildrenExt::visit_dfs(entity, &children, |edge| {
			commands
				.entity(edge)
				.remove::<(Interrupt, Running, RunResult)>();
		});
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	#[test]
	pub fn works() -> Result<()> {
		let mut app = App::new();
		app.add_plugins(BeetSystemsPlugin::<EcsModule, _>::default());

		let target = app.world_mut().spawn_empty().id();

		let tree =
			test_no_action_behavior_tree().spawn(app.world_mut(), target);

		tree.map(|entity| {
			app.world_mut().entity_mut(*entity).insert(Running);
		});

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(Some(&Running))
				.with_child(
					Tree::new(Some(&Running)).with_leaf(Some(&Running)),
				),
		)?;

		app.world_mut()
			.entity_mut(tree.children[1].value)
			.insert(Interrupt);

		app.update();

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(Some(&Running))
				.with_child(Tree::new(None).with_leaf(None)),
		)?;
		expect(tree.component_tree::<RunResult>(app.world())).to_be(
			Tree::new(None)
				.with_leaf(None)
				.with_child(Tree::new(None).with_leaf(None)),
		)?;

		Ok(())
	}
}
