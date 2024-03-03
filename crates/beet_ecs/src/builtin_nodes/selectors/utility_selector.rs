use super::*;
use crate::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::SystemConfigs;
use serde::Deserialize;
use serde::Serialize;

#[derive(Default)]
#[action(system=utility_selector)]
pub struct UtilitySelector;

pub enum UtilityInterruptRate {
	/// Interrupt every frame.
	Frame,
	/// Interrupt every time a score changes
	ScoreChanged,
}

//TODO interrupt if child score changes

pub fn utility_selector(
	mut commands: Commands,
	selectors: Query<(Entity, &UtilitySelector, &Edges), With<Running>>,
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
