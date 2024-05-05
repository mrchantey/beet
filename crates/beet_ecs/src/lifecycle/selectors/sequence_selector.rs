use super::*;
use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// An action that runs all of its children in order until one fails.
///
/// Logical AND - `RUN child1 THEN child2 etc`
///
/// If a child succeeds it will run the next child.
///
/// If there are no more children to run it will succeed.
///
/// If a child fails it will fail.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct SequenceSelector;
fn sequence_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &SequenceSelector, &Children), With<Running>>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (parent, _selector, children) in selectors.iter() {
		if any_child_running(children, &children_running) {
			// continue
			continue;
		}

		match first_child_result(children, &children_results) {
			Some((index, result)) => match result {
				&RunResult::Failure => {
					// finish
					commands.entity(parent).insert(RunResult::Failure);
				}
				&RunResult::Success => {
					if index == children.len() - 1 {
						// finish
						commands.entity(parent).insert(RunResult::Success);
					} else {
						// next
						commands.entity(children[index + 1]).insert(Running);
					}
				}
			},
			None => {
				// start
				commands.entity(children[0]).insert(Running);
			}
		}
	}
}

impl ActionMeta for SequenceSelector {
	fn graph_role(&self) -> GraphRole { GraphRole::Node }
}

impl ActionSystems for SequenceSelector {
	fn systems() -> SystemConfigs { sequence_selector.in_set(TickSet) }
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
		app.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<(SequenceSelector, InsertOnRun<RunResult>)>::default(
			),
		));

		let tree = SequenceSelector
			.child(InsertOnRun(RunResult::Success))
			.child(InsertOnRun(RunResult::Failure))
			.build(app.world_mut());

		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(Some(&Running))
				.with_leaf(None),
		)?;

		app.update();
		expect(tree.component_tree(app.world()))
			.to_be(Tree::new(Some(&Running)).with_leaf(None).with_leaf(None))?;

		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(None)
				.with_leaf(Some(&Running)),
		)?;

		app.update();
		expect(tree.component_tree(app.world()))
			.to_be(Tree::new(Some(&Running)).with_leaf(None).with_leaf(None))?;

		app.update();
		expect(tree.component_tree::<Running>(app.world()))
			.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&RunResult::Failure))
				.with_leaf(None)
				.with_leaf(None),
		)?;

		Ok(())
	}
}
