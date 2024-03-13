use crate::tests::utils::expect_tree;
use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

fn setup() -> (App, EntityGraph) {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let entity_graph = UtilitySelector
		.child((
			Score::default(),
			ConstantScore::new(Score::Fail),
			SetRunResult::failure(),
		))
		.child((
			Score::default(),
			ConstantScore::new(Score::Pass),
			SetRunResult::success(),
		))
		.spawn(&mut app, target);

	(app, entity_graph)
}


#[sweet_test]
pub fn works() -> Result<()> {
	let (mut app, entity_graph) = setup();

	app.update();
	expect_tree(
		&mut app,
		&entity_graph,
		Tree::new(Some(&Running))
			.with_leaf(None)
			.with_leaf(Some(&Running)),
	)?;

	// app.update();
	// expect_tree(
	// 	&mut app,
	// 	&entity_graph,
	// 	Tree::new(Some(&Running)).with_leaf(None).with_leaf(None),
	// )?;

	// app.update();
	// expect_tree::<Running>(
	// 	&mut app,
	// 	&entity_graph,
	// 	Tree::new(None).with_leaf(None).with_leaf(None),
	// )?;

	// expect_tree(
	// 	&mut app,
	// 	&entity_graph,
	// 	Tree::new(Some(&RunResult::Success))
	// 		.with_leaf(None)
	// 		.with_leaf(None),
	// )?;

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
