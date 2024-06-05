use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;


#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct QTableSelector<L: QTrainer> {
	pub evaluate: bool,
	pub learner: L,
}

fn q_table_selector<L: QTrainer>(
	query: Query<&QTableSelector<L>, With<Running>>,
) {
	for _selector in query.iter() {
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
