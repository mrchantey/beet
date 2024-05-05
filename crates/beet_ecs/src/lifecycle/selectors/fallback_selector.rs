use super::*;
use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


/// An action that runs all of its children in order until one succeeds.
///
/// Logical OR: `RUN child1 OTHERWISE child2 etc`
///
/// If a child succeeds it will succeed.
///
/// If the last child fails it will fail.
#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct FallbackSelector;

fn fallback_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &FallbackSelector, &Children), With<Running>>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (parent, _selector, children) in selectors.iter() {
		if any_child_running(children, &children_running) {
			continue;
		}

		match first_child_result(children, &children_results) {
			Some((index, result)) => match result {
				&RunResult::Success => {
					commands.entity(parent).insert(RunResult::Success);
				}
				&RunResult::Failure => {
					if index == children.len() - 1 {
						commands.entity(parent).insert(RunResult::Failure);
					} else {
						commands.entity(children[index + 1]).insert(Running);
					}
				}
			},
			None => {
				commands.entity(children[0]).insert(Running);
			}
		}
	}
}

impl ActionMeta for FallbackSelector {
	fn graph_role(&self) -> GraphRole { GraphRole::Child }
}

impl ActionSystems for FallbackSelector {
	fn systems() -> SystemConfigs { fallback_selector.in_set(TickSet) }
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
			ActionPlugin::<(FallbackSelector, InsertOnRun<RunResult>)>::default(
			),
		));

		let tree = FallbackSelector
			.child(InsertOnRun(RunResult::Failure))
			.child(InsertOnRun(RunResult::Success))
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
			Tree::new(Some(&RunResult::Success))
				.with_leaf(None)
				.with_leaf(None),
		)?;

		Ok(())
	}
}
