use super::*;
use crate::prelude::*;
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
#[derive_action]
#[action(child_components=[Score])]
#[action(graph_role=GraphRole::Child)]
pub struct SequenceSelector;
fn sequence_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &SequenceSelector, &Edges), With<Running>>,
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
					commands.entity(parent).insert(RunResult::Failure);
				}
				&RunResult::Success => {
					if index == children.len() - 1 {
						commands.entity(parent).insert(RunResult::Success);
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
