use super::*;
use crate::prelude::*;
use bevy::prelude::*;

/// A `Utility Selector` that observes the [`Score`] of each child and selects the highest to run.
///
/// It will return the result of the highest scoring child.
///
#[derive_action]
#[action(graph_role=GraphRole::Child, child_components=[Score])]
pub struct ScoreSelector;

pub enum UtilityInterruptRate {
	/// Interrupt every frame.
	Frame,
	/// Interrupt every time a score changes
	ScoreChanged,
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
	for (parent, _selector, children) in selectors.iter() {
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
			}
			// else no passing score, do nothing
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
		app.add_plugins(BeetSystemsPlugin::<EcsModule, _>::default());

		let tree = ScoreSelector
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
			.build(app.world_mut());

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
