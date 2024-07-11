use super::*;
use crate::prelude::*;
use bevy::prelude::*;

/// An action that runs all of its children in order until one fails.
/// - If a child succeeds it will run the next child.
/// - If there are no more children to run it will succeed.
/// - If a child fails it will fail.
#[derive(Debug, Default, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::ChildBehaviors)]
#[systems(sequence_selector.in_set(TickSet))]
pub struct SequenceSelector;
fn sequence_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &SequenceSelector, &Children), With<Running>>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (parent, _selector, children) in selectors.iter() {
		if any_child_running(children, &children_running) {
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

		let entity = app
			.world_mut()
			.spawn((Running, SequenceSelector))
			.with_children(|parent| {
				parent.spawn(InsertOnRun(RunResult::Success));
				parent.spawn(InsertOnRun(RunResult::Failure));
			})
			.id();

		let tree = EntityTree::new_with_world(entity, app.world());

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
