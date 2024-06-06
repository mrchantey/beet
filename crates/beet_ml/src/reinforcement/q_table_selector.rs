use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;



/// An action used for training, evaluating and running QTable agents.
/// - If a child succeeds, evaluate reward and select next action.
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct QTableSelector<L: QTrainer> {
	pub evaluate: bool,
	pub learner: L,
	pub current_episode: usize,
	pub current_step: usize,
}

fn q_table_selector<L: QTrainer>(
	mut commands: Commands,
	mut agents: Query<(&L::State, &mut L::Action, &Reward)>,
	mut query: Query<
		(Entity, &TargetAgent, &mut QTableSelector<L>, &Children),
		With<Running>,
	>,
	children_running: Query<(), With<Running>>,
	children_results: Query<&RunResult>,
) {
	for (action_entity, agent, mut selector, children) in query.iter_mut() {
		let Ok((state, mut action, reward)) = agents.get_mut(**agent) else {
			continue;
		};

		if any_child_running(children, &children_running) {
			continue;
		}

		selector.current_step += 1;

		match first_child_result(children, &children_results) {
			Some((index, result)) => match result {
				&RunResult::Failure => {
					// end episode
					commands.entity(action_entity).insert(RunResult::Failure);
					continue;
				}
				&RunResult::Success => {

					// *action = selector.next_action(state, reward);
					// true
					// if index == children.len() - 1 {
					// 	// finish
					// 	commands.entity(parent).insert(RunResult::Success);
					// } else {
					// 	// next
					// 	commands.entity(children[index + 1]).insert(Running);
					// }
				}
			},
			None => {
				// start
				// commands.entity(children[0]).insert(Running);
			}
		}
	}
}

impl<L: QTrainer> ActionMeta for QTableSelector<L> {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl<L: QTrainer> ActionSystems for QTableSelector<L> {
	fn systems() -> SystemConfigs { q_table_selector::<L>.in_set(TickSet) }
}
