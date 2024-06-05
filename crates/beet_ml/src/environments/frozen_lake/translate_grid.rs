use crate::prelude::ActionSpace;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use rand::Rng;
use strum::EnumCount;
use strum::EnumIter;
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
	EnumIter,
	EnumCount,
)]
pub enum TranslateGridDirection {
	#[default]
	Up,
	Right,
	Down,
	Left,
}

impl Into<IVec2> for TranslateGridDirection {
	fn into(self) -> IVec2 {
		match self {
			Self::Up => IVec2::new(0, 1),
			Self::Right => IVec2::new(1, 0),
			Self::Down => IVec2::new(0, -1),
			Self::Left => IVec2::new(-1, 0),
		}
	}
}

impl From<usize> for TranslateGridDirection {
	fn from(value: usize) -> Self {
		match value {
			0 => Self::Up,
			1 => Self::Right,
			2 => Self::Down,
			3 => Self::Left,
			_ => unreachable!(),
		}
	}
}

impl TranslateGridDirection {
	pub fn as_slippery(&self) -> Self {
		let mut rng = rand::thread_rng();
		match rng.gen_range(0..3) {
			0 => self.clone(),
			1 => self.rotate_left(),
			2 => self.rotate_right(),
			_ => unreachable!(),
		}
	}
	pub fn rotate_left(&self) -> Self {
		match self {
			Self::Up => Self::Left,
			Self::Right => Self::Up,
			Self::Down => Self::Right,
			Self::Left => Self::Down,
		}
	}
	pub fn rotate_right(&self) -> Self {
		match self {
			Self::Up => Self::Right,
			Self::Right => Self::Down,
			Self::Down => Self::Left,
			Self::Left => Self::Up,
		}
	}

	pub fn with_adjacents(&self) -> Vec<Self> {
		match self {
			Self::Up => vec![Self::Left, Self::Up, Self::Right],
			Self::Right => vec![Self::Up, Self::Right, Self::Down],
			Self::Down => vec![Self::Right, Self::Down, Self::Left],
			Self::Left => vec![Self::Down, Self::Left, Self::Up],
		}
	}
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

// impl DiscreteSpace for TranslateGridDirection {
// 	const LEN: usize = Self::COUNT;
// }

impl ActionSpace for TranslateGridDirection {
	fn sample() -> Self {
		match rand::thread_rng().gen_range(0..4) {
			0 => Self::Up,
			1 => Self::Right,
			2 => Self::Down,
			3 => Self::Left,
			_ => unreachable!(),
		}
	}
}
