use super::*;
use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemConfigs;
use serde::Deserialize;
use serde::Serialize;

/// A node that runs all of its children in order until one fails.
///
/// If a child succeeds it will run the next child.
///
/// If there are no more children to run it will succeed.
///
/// If a child fails it will fail.
#[derive(Default)]
#[action(system=sequence)]
pub struct SequenceSelector;
pub fn sequence(
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
