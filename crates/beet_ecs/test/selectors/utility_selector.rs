use beet_ecs::prelude::*;
use bevy::prelude::*;
use sweet::*;

fn setup() -> (App, EntityTree) {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let tree = UtilitySelector
		.child((
			Score::default(),
			SetOnStart(Score::Fail),
			InsertOnRun(RunResult::Failure),
		))
		.child((
			Score::default(),
			SetOnStart(Score::Pass),
			InsertOnRun(RunResult::Success),
		))
		.spawn(&mut app.world, target);

	(app, tree)
}


#[sweet_test]
pub fn works() -> Result<()> {
	let (mut app, tree) = setup();

	app.update();
	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&Running))
			.with_leaf(None)
			.with_leaf(Some(&Running)),
	)?;


	app.update();
	expect(tree.component_tree(&app.world)).to_be(
		Tree::new(Some(&RunResult::Success))
			.with_leaf(None)
			.with_leaf(Some(&RunResult::Success)),
	)?;

	expect(tree.component_tree::<Running>(&app.world))
		.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;

	app.update();

	expect(tree.component_tree::<RunResult>(&app.world))
		.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;

	Ok(())
}
// #[sweet_test]
// pub fn interrupts() -> Result<()> {
// 	let (mut app, entity_graph) = setup();
// 	app.update();

// 	let child = entity_graph.clone().into_tree().children[1].value;
// 	app.world
// 		.entity_mut(child)
// 		.insert(ConstantScore::new(Score::Pass));

// 	app.update();
// 	expect_tree(
// 		&mut app,
// 		&entity_graph,
// 		Tree::new(Some(&Running))
// 			.with_leaf(None)
// 			.with_leaf(Some(&Running)),
// 	)?;

// 	Ok(())
// }
