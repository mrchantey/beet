use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(BeetSystemsPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let tree = FallbackSelector
		.child(InsertOnRun(RunResult::Failure))
		.child(InsertOnRun(RunResult::Success))
		.spawn(&mut app.world, target);

	app.update();
	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&Running))
			.with_leaf(Some(&Running))
			.with_leaf(None),
	)?;

	app.update();
	expect(tree.component_tree(&app.world))
		.to_be(Tree::new(Some(&Running)).with_leaf(None).with_leaf(None))?;

	app.update();
	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&Running))
			.with_leaf(None)
			.with_leaf(Some(&Running)),
	)?;

	app.update();
	expect(tree.component_tree(&app.world))
		.to_be(Tree::new(Some(&Running)).with_leaf(None).with_leaf(None))?;

	app.update();
	expect(tree.component_tree::<Running>(&app.world))
		.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;
	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&RunResult::Success))
			.with_leaf(None)
			.with_leaf(None),
	)?;

	Ok(())
}
