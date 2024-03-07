use crate::tests::utils::expect_tree;
use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

fn setup() -> (App, EntityGraph) {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let action_graph = BehaviorTree::<EcsNode>::new(UtilitySelector)
		.with_child((SetScore::new(Score::Fail), SetRunResult::failure()))
		.with_child((SetScore::new(Score::Pass), SetRunResult::success()))
		.into_action_graph();

	let entity_graph = action_graph.spawn(&mut app.world, target);
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

	app.update();
	expect_tree(
		&mut app,
		&entity_graph,
		Tree::new(Some(&Running)).with_leaf(None).with_leaf(None),
	)?;

	app.update();
	expect_tree::<Running>(
		&mut app,
		&entity_graph,
		Tree::new(None).with_leaf(None).with_leaf(None),
	)?;

	expect_tree(
		&mut app,
		&entity_graph,
		Tree::new(Some(&RunResult::Success))
			.with_leaf(None)
			.with_leaf(None),
	)?;

	Ok(())
}
// #[sweet_test]
// pub fn interrupts() -> Result<()> {
// 	let (mut app, entity_graph) = setup();
// 	app.update();

// 	let child = entity_graph.clone().into_tree().children[1].value;
// 	app.world
// 		.entity_mut(child)
// 		.insert(SetScore::new(Score::Pass));

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
