use super::*;
use crate::prelude::*;
use bevy::prelude::*;

/// Run children in parallel until one finishes.
#[derive(Debug, Default, Clone, PartialEq, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::ChildBehaviors)]
#[systems(parallel_selector.in_set(TickSet))]
pub struct ParallelSelector;

fn parallel_selector(
	mut commands: Commands,
	actions: Query<(Entity, &ParallelSelector, &Children), With<Running>>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (parent, _selector, children) in actions.iter() {
		match first_child_result(children, &children_results) {
			Some((_, result)) => {
				// finish
				commands.entity(parent).insert(*result);
				for child in children.iter() {
					commands.entity(*child).insert(Interrupt);
				}
			}
			None => {
				if any_child_running(children, &children_running) {
					// continue
					continue;
				} else {
					// start
					for child in children.iter() {
						commands.entity(*child).insert(Running);
					}
				}
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
		pretty_env_logger::try_init().ok();
		let mut app = App::new();
		app.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<ParallelSelector>::default(),
		));

		let entity = app
			.world_mut()
			.spawn((Running, ParallelSelector))
			.with_children(|parent| {
				parent.spawn_empty();
				parent.spawn_empty();
			})
			.id();

		let tree = EntityTree::new_with_world(entity, app.world());

		app.update();
		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(Some(&Running))
				.with_leaf(Some(&Running)),
		)?;

		app.update();

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&Running))
				.with_leaf(Some(&Running))
				.with_leaf(Some(&Running)),
		)?;

		app.world_mut()
			.entity_mut(tree.children[0].value)
			.insert(RunResult::Success);

		app.update();

		expect(tree.component_tree(app.world())).to_be(
			Tree::new(Some(&RunResult::Success))
				.with_leaf(None)
				.with_leaf(None),
		)?;

		expect(tree.component_tree::<Running>(app.world()))
			.to_be(Tree::new(None).with_leaf(None).with_leaf(None))?;

		Ok(())
	}
}
