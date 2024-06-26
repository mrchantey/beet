use crate::prelude::*;
use bevy::prelude::*;


pub fn any_child_running(
	children: &Children,
	children_running: &Query<(), With<Running>>,
) -> bool {
	children
		.iter()
		.any(|child| children_running.contains(*child))
}

pub fn first_child_result<'a>(
	children: &Children,
	children_results: &'a Query<&RunResult>,
) -> Option<(usize, &'a RunResult)> {
	children.iter().enumerate().find_map(|(index, child)| {
		if let Ok(result) = children_results.get(*child) {
			Some((index, result))
		} else {
			None
		}
	})
}

pub fn any_child_score_changed(
	children: &Children,
	children_scores_changed: &Query<(), Changed<Score>>,
) -> bool {
	children
		.iter()
		.any(|child| children_scores_changed.contains(*child))
}


pub fn highest_score(
	children: &Children,
	children_scores: &Query<(Entity, &Score)>,
) -> Option<(Entity, Score)> {
	children.iter().fold(None, |prev, child| {
		if let Ok((child, score)) = children_scores.get(*child) {
			if let Some((_, last_score)) = prev {
				if *score > last_score {
					return Some((child, *score));
				}
			} else {
				return Some((child, *score));
			}
		}
		prev
	})
}
