use crate::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use bevy::utils::HashMap;
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
pub enum FrozenLakeTile {
	Agent,
	#[default]
	Ice,
	Hole,
	Goal,
}

impl FrozenLakeTile{
	pub fn reward(&self) -> f32 {
		match self {
			Self::Goal => 1.0,
			Self::Hole => 0.0,
			_ => 0.0,
		}
	}
}

impl FrozenLakeTile {
	pub fn is_terminal(&self) -> bool {
		matches!(self, Self::Goal | Self::Hole)
	}
}
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Component)]
pub struct FrozenLakeMap<const L: usize> {
	tiles: [FrozenLakeTile; L],
	width: usize,
	height: usize,
}

impl<const L: usize> FrozenLakeMap<L> {
	pub fn new(
		width: usize,
		height: usize,
		tiles: [FrozenLakeTile; L],
	) -> Self {
		Self {
			tiles,
			width,
			height,
		}
	}

	fn index_to_position(&self, index: usize) -> UVec2 {
		UVec2::new((index % self.width) as u32, (index / self.width) as u32)
	}

	fn position_to_index(&self,position: UVec2) -> usize {
		(position.y as usize) * self.width + position.x as usize
	}

	fn position_to_tile(&self, position: UVec2) -> FrozenLakeTile {
		self.tiles[self.position_to_index(position)]
	}

	pub fn tiles(&self) -> &[FrozenLakeTile; L] { &self.tiles }
	pub fn width(&self) -> usize { self.width }
	pub fn height(&self) -> usize { self.height }
	pub fn tiles_with_positions(
		&self,
	) -> impl Iterator<Item = (UVec2, &FrozenLakeTile)> {
		self.tiles
			.iter()
			.enumerate()
			.map(move |(i, tile)| (self.index_to_position(i), tile))
	}

	#[rustfmt::skip]
	fn out_of_bounds(&self, pos: IVec2) -> bool {
		pos.x < 0
			|| pos.y < 0 
			|| pos.x >= self.width as i32
			|| pos.y >= self.height as i32
	}

	pub fn try_translate(
		&self,
		position: UVec2,
		direction: TranslateGridDirection,
	) -> Option<(UVec2, FrozenLakeTile)> {
		let direction: IVec2= direction.into();
		let new_pos = IVec2::new(position.x as i32 + direction.x,position.y as i32 + direction.y);
		if self.out_of_bounds(new_pos) {
			None
		}else{
			let new_pos = new_pos.try_into().expect("already checked in bounds");
			Some((new_pos, self.position_to_tile(new_pos)))
		}
	}

	pub fn agent_position(&self) -> Option<UVec2> {
		self.tiles.iter().enumerate().find_map(|(i, &tile)| {
			if tile == FrozenLakeTile::Agent {
				Some(self.index_to_position(i))
			} else {
				None
			}
		})
	}
	// pub fn get_tile(&self, x: usize, y: usize) -> Option<&FrozenLakeTile> {
	// 	if x >= D::WIDTH || y >= D::HEIGHT {
	// 		return None;
	// 	}
	// 	Some(&self.tiles[y * D::WIDTH + x])
	// }

	pub fn transition_outcomes(
		&self,
	) -> HashMap<(UVec2, TranslateGridDirection), TransitionOutcome> {
		let mut outcomes = HashMap::new();
		for (pos, tile) in self.tiles_with_positions() {
			for action in TranslateGridDirection::iter() {
				let outcome = if tile.is_terminal() {
					// early exit, cannot move from terminal tile
					TransitionOutcome {
						reward: 0.0,
						pos,
						is_terminal: true,
					}
				} else if let Some((new_pos, new_tile)) =
					self.try_translate(pos, action)
				{
					// yes you can go here
					TransitionOutcome {
						reward: new_tile.reward(),
						pos: new_pos,
						is_terminal: new_tile.is_terminal(),
					}
				} else {
					// stay where you are
					TransitionOutcome {
						reward: 0.0,
						pos,
						is_terminal: false,
					}
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


// impl<const WIDTH:usize, const HEIGHT:usize>RlEnvironment for FrozenLakeMap<WIDTH, HEIGHT>
// where
// 	[(); WIDTH * HEIGHT]:,
// {
// 	fn observation_space(&self) -> impl ObservationSpace { self }
// 	fn action_space(&self) -> impl ActionSpace { self }
// 	fn reset(&mut self) {
// 		*self = Self::def
// 	}
// }

impl FrozenLakeMap<16> {
	#[rustfmt::skip]
	pub fn default_four_by_four() -> Self {
		Self {
			width: 4,
			height: 4,
			//https://github.com/openai/gym/blob/dcd185843a62953e27c2d54dc8c2d647d604b635/gym/envs/toy_text/frozen_lake.py#L17
			tiles: [
				//row 1
				FrozenLakeTile::Agent,	FrozenLakeTile::Ice,	FrozenLakeTile::Ice,	FrozenLakeTile::Ice,
				//row 2
				FrozenLakeTile::Ice,		FrozenLakeTile::Hole,	FrozenLakeTile::Ice,	FrozenLakeTile::Hole,
				//row 3
				FrozenLakeTile::Ice,		FrozenLakeTile::Ice,	FrozenLakeTile::Ice,	FrozenLakeTile::Hole,
				//row 4
				FrozenLakeTile::Hole,		FrozenLakeTile::Ice,	FrozenLakeTile::Ice,	FrozenLakeTile::Goal,
			],
		}
	}
}


impl FrozenLakeMap<64> {
	pub fn default_eight_by_eight() -> Self {
		Self {
			width: 8,
			height: 8,
			//https://github.com/openai/gym/blob/dcd185843a62953e27c2d54dc8c2d647d604b635/gym/envs/toy_text/frozen_lake.py#L17
			tiles: [
				//row 1
				FrozenLakeTile::Agent,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 2
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 3
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 4
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 5
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 6
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				//row 7
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				//row 8
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Goal,
			],
		}
	}
}
