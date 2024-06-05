use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use serde::Deserialize;
use serde::Serialize;
use strum::IntoEnumIterator;


#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	Component,
)]
pub enum FrozenLakeCell {
	Agent,
	#[default]
	Ice,
	Hole,
	Goal,
}

impl FrozenLakeCell {
	pub fn reward(&self) -> f32 {
		match self {
			Self::Goal => 1.0,
			Self::Hole => 0.0,
			_ => 0.0,
		}
	}
}

impl FrozenLakeCell {
	pub fn is_terminal(&self) -> bool {
		matches!(self, Self::Goal | Self::Hole)
	}
}


/// Define an intial state for a [`FrozenLakeEnv`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Component)]
pub struct FrozenLakeMap<const L: usize> {
	cells: [FrozenLakeCell; L],
	width: usize,
	height: usize,
}

impl<const L: usize> FrozenLakeMap<L> {
	pub fn new(
		width: usize,
		height: usize,
		cells: [FrozenLakeCell; L],
	) -> Self {
		Self {
			cells,
			width,
			height,
		}
	}

	fn index_to_position(&self, index: usize) -> UVec2 {
		UVec2::new((index % self.width) as u32, (index / self.width) as u32)
	}

	fn position_to_index(&self, position: UVec2) -> usize {
		(position.y as usize) * self.width + position.x as usize
	}

	fn position_to_cell(&self, position: UVec2) -> FrozenLakeCell {
		self.cells[self.position_to_index(position)]
	}

	pub fn cells(&self) -> &[FrozenLakeCell; L] { &self.cells }
	pub fn width(&self) -> usize { self.width }
	pub fn height(&self) -> usize { self.height }
	pub fn cells_with_positions(
		&self,
	) -> impl Iterator<Item = (UVec2, &FrozenLakeCell)> {
		self.cells
			.iter()
			.enumerate()
			.map(move |(i, cell)| (self.index_to_position(i), cell))
	}

	fn out_of_bounds(&self, pos: IVec2) -> bool {
		pos.x < 0
			|| pos.y < 0 || pos.x >= self.width as i32
			|| pos.y >= self.height as i32
	}

	pub fn try_transition(
		&self,
		position: UVec2,
		direction: TranslateGridDirection,
	) -> Option<TransitionOutcome> {
		let direction: IVec2 = direction.into();
		let new_pos = IVec2::new(
			position.x as i32 + direction.x,
			position.y as i32 + direction.y,
		);
		if self.out_of_bounds(new_pos) {
			None
		} else {
			let new_pos =
				new_pos.try_into().expect("already checked in bounds");
			let new_cell = self.position_to_cell(new_pos);
			Some(TransitionOutcome {
				reward: new_cell.reward(),
				pos: new_pos,
				is_terminal: new_cell.is_terminal(),
			})
		}
	}

	pub fn agent_position(&self) -> Option<UVec2> {
		self.cells.iter().enumerate().find_map(|(i, &cell)| {
			if cell == FrozenLakeCell::Agent {
				Some(self.index_to_position(i))
			} else {
				None
			}
		})
	}

	pub fn transition_outcomes(
		&self,
	) -> HashMap<(UVec2, TranslateGridDirection), TransitionOutcome> {
		let mut outcomes = HashMap::new();
		for (pos, cell) in self.cells_with_positions() {
			for action in TranslateGridDirection::iter() {
				let outcome = if cell.is_terminal() {
					// early exit, cannot move from terminal cell
					TransitionOutcome {
						reward: 0.0,
						pos,
						is_terminal: true,
					}
				} else {
					// yes you can go here
					self.try_transition(pos, action).unwrap_or(
						// stay where you are
						TransitionOutcome {
							reward: 0.0,
							pos,
							is_terminal: false,
						},
					)
				};
				outcomes.insert((pos, action), outcome);
			}
		}

		outcomes
	}
}

// impl<const L: usize> Space for FrozenLakeMap<L> {
// const LEN: usize = L;
// type Value = FrozenLakeTile;
// fn shape(&self) -> SpaceShape { SpaceShape::Discrete(L) }
// // fn len(&self) -> usize { WIDTH * HEIGHT }
// fn sample(&self) -> Self::Value { self.tiles[0] }
// }

impl FrozenLakeMap<16> {
	#[rustfmt::skip]
	pub fn default_four_by_four() -> Self {
		Self {
			width: 4,
			height: 4,
			//https://github.com/openai/gym/blob/dcd185843a62953e27c2d54dc8c2d647d604b635/gym/envs/toy_text/frozen_lake.py#L17
			cells: [
				//row 1
				FrozenLakeCell::Agent,	FrozenLakeCell::Ice,	FrozenLakeCell::Ice,	FrozenLakeCell::Ice,
				//row 2
				FrozenLakeCell::Ice,		FrozenLakeCell::Hole,	FrozenLakeCell::Ice,	FrozenLakeCell::Hole,
				//row 3
				FrozenLakeCell::Ice,		FrozenLakeCell::Ice,	FrozenLakeCell::Ice,	FrozenLakeCell::Hole,
				//row 4
				FrozenLakeCell::Hole,		FrozenLakeCell::Ice,	FrozenLakeCell::Ice,	FrozenLakeCell::Goal,
			],
		}
	}
}


impl FrozenLakeMap<64> {
	#[rustfmt::skip]
	pub fn default_eight_by_eight() -> Self {
		Self {
			width: 8,
			height: 8,
			//https://github.com/openai/gym/blob/dcd185843a62953e27c2d54dc8c2d647d604b635/gym/envs/toy_text/frozen_lake.py#L17
			cells: [
				//row 1
				FrozenLakeCell::Agent,	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice,
				//row 2
				FrozenLakeCell::Ice, 		FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice,
				//row 3
				FrozenLakeCell::Ice, 		FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice,
				//row 4
				FrozenLakeCell::Ice, 		FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice,
				//row 5
				FrozenLakeCell::Ice, 		FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice,
				//row 6
				FrozenLakeCell::Ice, 		FrozenLakeCell::Hole, FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice,
				//row 7
				FrozenLakeCell::Ice, 		FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice,
				//row 8
				FrozenLakeCell::Ice, 		FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Hole, FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Ice, 	FrozenLakeCell::Goal,
			],
		}
	}
}
