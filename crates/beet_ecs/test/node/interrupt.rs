use crate::tests::utils::expect_tree;
use beet_ecs::prelude::*;
use bevy_app::App;
use sweet::*;

#[sweet_test]
pub fn works() -> Result<()> {
	let mut app = App::new();
	app.add_plugins(ActionPlugin::<EcsNode, _>::default());

	let target = app.world.spawn_empty().id();

	let action_graph: BehaviorGraph<EcsNode> =
		BehaviorGraph::<EcsNode>::from_tree(
			Tree::new(vec![EmptyAction.into()].into())
				.with_leaf(vec![EmptyAction.into()].into())
				.with_child(
					Tree::new(vec![EmptyAction.into()].into())
						.with_leaf(vec![EmptyAction.into()].into()),
				),
		);

	let entity_graph = action_graph.spawn(&mut app.world, target);

	for entity in entity_graph.node_weights() {
		app.world.entity_mut(*entity).insert(Running);
	}

	expect_tree(
		&mut app,
		&entity_graph,
		Tree::new(Some(&Running))
			.with_leaf(Some(&Running))
			.with_child(Tree::new(Some(&Running)).with_leaf(Some(&Running))),
	)?;

	let entity = &entity_graph.0.clone().into_tree().children[1].value;
	app.world.entity_mut(*entity).insert(Interrupt);

	app.update();

	expect_tree(
		&mut app,
		&entity_graph,
		Tree::new(Some(&Running))
			.with_leaf(Some(&Running))
			.with_child(Tree::new(None).with_leaf(None)),
	)?;
	expect_tree::<Interrupt>(
		&mut app,
		&entity_graph,
		Tree::new(None)
			.with_leaf(None)
			.with_child(Tree::new(None).with_leaf(None)),
	)?;

	Ok(())
}
