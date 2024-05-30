use crate::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


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

pub fn index_to_position(index: usize, width: usize) -> UVec2 {
	UVec2::new((index % width) as u32, (index / width) as u32)
}

pub fn position_to_index(position: UVec2, width: usize) -> usize {
	(position.y as usize) * width + position.x as usize
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
	pub fn tiles(&self) -> &[FrozenLakeTile; L] { &self.tiles }

	pub fn width(&self) -> usize { self.width }
	pub fn height(&self) -> usize { self.height }
	pub fn agent_position(&self) -> Option<UVec2> {
		self.tiles.iter().enumerate().find_map(|(i, &tile)| {
			if tile == FrozenLakeTile::Agent {
				Some(index_to_position(i, self.width))
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
}

impl<const L: usize> Space for FrozenLakeMap<L> {
	const LEN: usize = L;
	type Value = FrozenLakeTile;
	fn shape(&self) -> SpaceShape { SpaceShape::Discrete(L) }
	// fn len(&self) -> usize { WIDTH * HEIGHT }
	fn sample(&self) -> Self::Value { self.tiles[0] }
}


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
	pub fn default_four_by_four() -> Self {
		Self {
			width: 4,
			height: 4,
			//https://github.com/openai/gym/blob/dcd185843a62953e27c2d54dc8c2d647d604b635/gym/envs/toy_text/frozen_lake.py#L17
			tiles: [
				//row 1
				FrozenLakeTile::Agent,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				//row 2
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				//row 3
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Hole,
				//row 4
				FrozenLakeTile::Hole,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Ice,
				FrozenLakeTile::Goal,
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
