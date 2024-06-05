// use crate::prelude::*;
// use beet_ecs::prelude::*;
// use bevy::ecs::schedule::SystemConfigs;
// use bevy::prelude::*;


// #[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
// #[reflect(Default, Component, ActionMeta)]
// pub struct QTableSelector<T: Default + Reflect + QSource> {
// 	pub table: T,
// 	pub evaluate: bool,
// 	pub trainer: QTableTrainer,
// }

// fn q_table_selector<T: Default + Reflect + QSource>(
// 	query: Query<&QTableSelector<T>, With<Running>>,
// ) {
// 	for selector in query.iter() {

// 		// log::info!("Running - {:?}", q_table_selector);
// 	}
// }

// impl<T: Default + Reflect + QSource> ActionMeta for QTableSelector<T> {
// 	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
// }

// impl<T: Default + Reflect + QSource> ActionSystems for QTableSelector<T> {
// 	fn systems() -> SystemConfigs { q_table_selector::<T>.in_set(TickSet) }
// }
