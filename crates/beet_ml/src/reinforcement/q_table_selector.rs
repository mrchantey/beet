use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct QTableSelector<L: QTrainer> {
	pub evaluate: bool,
	pub learner: L,
	pub current_episode: usize,
	pub current_step: usize,
}

fn q_table_selector<L: QTrainer>(
	mut agents: Query<(&L::State, &mut L::Action, &Reward)>,
	query: Query<(&TargetAgent, &QTableSelector<L>), With<Running>>,
) {
	for (agent, selector) in query.iter() {
		let Ok((state, mut action, reward)) = agents.get_mut(**agent) else {
			continue;
		};

		// *action = selector.next_action(state, reward);

		// if selector.
		// log::info!("Running - {:?}", q_table_selector);
	}
}

impl<L: QTrainer> ActionMeta for QTableSelector<L> {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl<L: QTrainer> ActionSystems for QTableSelector<L> {
	fn systems() -> SystemConfigs { q_table_selector::<L>.in_set(TickSet) }
}
