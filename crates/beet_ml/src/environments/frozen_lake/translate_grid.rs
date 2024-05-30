use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use strum::VariantArray;

#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	Reflect,
	PartialEq,
	Eq,
	Hash,
	Component,
	VariantArray,
)]
pub enum TranslateGridDirection {
	#[default]
	Up,
	Right,
	Down,
	Left,
}



#[derive(Debug, Default, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct TranslateGrid {
	pub direction: TranslateGridDirection,
}

fn translate_grid(query: Query<&TranslateGrid, With<Running>>) {
	for translate_grid in query.iter() {
		log::info!("Running - {:?}", translate_grid);
	}
}

impl ActionMeta for TranslateGrid {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl ActionSystems for TranslateGrid {
	fn systems() -> SystemConfigs { translate_grid.in_set(TickSet) }
}


impl Space for TranslateGrid {
	type Value = TranslateGridDirection;
	const LEN: usize = 4;
	fn shape(&self) -> SpaceShape { SpaceShape::Discrete(4) }
	// fn len(&self) -> usize { 4 }
	fn sample(&self) -> Self::Value { Self::Value::default() }
}
