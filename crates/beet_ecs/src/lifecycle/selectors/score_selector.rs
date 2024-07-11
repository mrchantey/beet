use super::*;
use crate::prelude::*;
use bevy::prelude::*;


/// A `Utility Selector` that observes the [`Score`] of each child and selects the highest to run.
///
/// It will return the result of the highest scoring child.
///
#[derive(Debug, Default, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::ChildBehaviors)]
#[systems(score_selector.in_set(TickSet))]
pub struct ScoreSelector {
	/// Remove the score component from children when one is selected. This is useful if
	/// the selector should only run once.
	pub consume_scores: bool,
}

impl ScoreSelector {
	pub fn new(consume_scores: bool) -> Self { Self { consume_scores } }
	pub fn consuming() -> Self {
		Self {
			consume_scores: true,
		}
	}
}

//TODO interrupt if child score changes

fn score_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &ScoreSelector, &Children), With<Running>>,
	children_scores: Query<(Entity, &Score)>,
	children_scores_changed: Query<(), Changed<Score>>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (parent, selector, children) in selectors.iter() {
		// if a child has finished, return
		if let Some((_, result)) =
			first_child_result(children, &children_results)
		{
			commands.entity(parent).insert(result.clone());
			continue;
		}

		// recalculate if a score changed or no children are running
		// TODO this could be further optimized
		if any_child_score_changed(children, &children_scores_changed)
			|| false == any_child_running(children, &children_running)
		{
			if let Some((highest_child, _)) =
				highest_score(children, &children_scores)
			{
				// continue if highest score already running
				if children_running.contains(highest_child) {
					continue;
				}

				// interrupt other running children
				for child in children
					.iter()
					// .filter(|child| **child != highest_child)
					.filter(|child| children_running.contains(**child))
				{
					commands.entity(*child).insert(Interrupt);
				}

				// run highest score
				commands.entity(highest_child).insert(Running);

				if selector.consume_scores {
					for child in children.iter() {
						commands.entity(*child).remove::<Score>();
					}
				}
			} else {
				// no highest score, do nothing
				continue;
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use anyhow::Result;
	use bevy::prelude::*;
	use sweet::*;

	fn setup() -> (App, EntityTree) {
		let mut app = App::new();
		app.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<(
				ScoreSelector,
				SetOnSpawn<Score>,
				InsertOnRun<RunResult>,
			)>::default(),
		));

		let entity = app
			.world_mut()
			.spawn((Running, ScoreSelector::default()))
			.with_children(|parent| {
				parent.spawn((
					Score::default(),
					SetOnSpawn(Score::Fail),
					InsertOnRun(RunResult::Failure),
				));
				parent.spawn((
					Score::default(),
					SetOnSpawn(Score::Pass),
					InsertOnRun(RunResult::Success),
				));
			})
			.id();
		let tree = EntityTree::new_with_world(entity, app.world());

		(app, tree)
	}


	#[test]
	pub fn works() -> Result<()> {
		let (mut app, tree) = setup();

		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(None)
				.with_leaf(Some(&Running)),
		)?;


		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&RunResult::Success))
				.with_leaf(None)
				.with_leaf(Some(&RunResult::Success)),
		)?;

		expect(tree.component_tree::<Running>(app.world()))
			.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;

		app.update();

		expect(tree.component_tree::<RunResult>(app.world()))
			.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;

		Ok(())
	}
	// #[test]
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
}
