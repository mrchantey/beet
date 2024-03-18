use crate::tests::action::test_no_action_behavior_tree;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let tree = test_no_action_behavior_tree().spawn(&mut app.world, target);

	tree.visit_dfs(&mut |entity| {
		app.world.entity_mut(entity).insert(Running);
	});

	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&Running))
			.with_leaf(Some(&Running))
			.with_child(Tree::new(Some(&Running)).with_leaf(Some(&Running))),
	)?;

	app.world
		.entity_mut(tree.children[1].value)
		.insert(Interrupt);

	app.update();

	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&Running))
			.with_leaf(Some(&Running))
			.with_child(Tree::new(None).with_leaf(None)),
	)?;
	expect(tree.component_tree::<RunResult>(&app.world)).to_be(
		Tree::new(None)
			.with_leaf(None)
			.with_child(Tree::new(None).with_leaf(None)),
	)?;

	Ok(())
}
