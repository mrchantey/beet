use super::*;
use crate::prelude::*;
use bevy::prelude::*;

/// Run children in parallel until one finishes.
#[derive_action]
#[action(graph_role=GraphRole::Child)]
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
		pretty_env_logger::init();
		let mut app = App::new();
		app.add_plugins(BeetSystemsPlugin::<EcsModule, _>::default());

		let tree = ParallelSelector.child(()).child(()).build(app.world_mut());

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
