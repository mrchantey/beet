use super::frozen_lake_map::FrozenLakeMap;
use crate::prelude::ActionSpace;
use bevy::prelude::*;
use rand::rngs::StdRng;
use rand::Rng;
use strum::EnumCount;
use strum::EnumIter;
use strum::VariantArray;

#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	Hash,
	PartialEq,
	Eq,
	Deref,
	DerefMut,
	Component,
	Reflect,
)]
pub struct GridPos(pub UVec2);

impl GridPos {
	pub fn new(pos: UVec2) -> Self { Self(pos) }
	// this is saturating add signed?
	// pub fn sat_add(&mut self, other: IVec2) {
	// 	fn add(u: u32, i: i32) -> u32 {
	// 		if i.is_negative() {
	// 			u - i.wrapping_abs() as u32
	// 		} else {
	// 			u + i as u32
	// 		}
	// 	}
	// 	self.0.x = add(self.0.x, other.x);
	// 	self.0.y = add(self.0.y, other.y);
	// }
}

impl From<UVec2> for GridPos {
	fn from(pos: UVec2) -> Self { Self(pos) }
}

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
pub enum GridDirection {
	#[default]
	Up,
	Right,
	Down,
	Left,
}


impl Into<IVec2> for GridDirection {
	fn into(self) -> IVec2 {
		match self {
			Self::Up => IVec2::new(0, 1),
			Self::Right => IVec2::new(1, 0),
			Self::Down => IVec2::new(0, -1),
			Self::Left => IVec2::new(-1, 0),
		}
	}
}

impl Into<Vec3> for GridDirection {
	fn into(self) -> Vec3 {
		let v: IVec2 = self.into();
		Vec3::new(v.x as f32, 0., v.y as f32)
	}
}

impl GridDirection {
	pub fn as_slippery(&self, rng: &mut StdRng) -> Self {
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
}


impl ActionSpace for GridDirection {
	fn sample(rng: &mut impl Rng) -> Self {
		match rng.gen_range(0..4) {
			0 => Self::Up,
			1 => Self::Right,
			2 => Self::Down,
			3 => Self::Left,
			_ => unreachable!(),
		}
	}
}


#[derive(Debug, Clone, Component)]
pub struct GridToWorld {
	pub map_width: f32,
	pub cell_width: f32,
	pub offset: Vec3,
}

impl GridToWorld {
	pub fn from_frozen_lake_map(grid: &FrozenLakeMap, map_width: f32) -> Self {
		let cell_width = map_width / grid.width() as f32;
		let offset = cell_width * 0.5
			+ Vec3::new(grid.width() as f32, 0., grid.height() as f32) * -0.5;

		Self {
			map_width,
			cell_width,
			offset,
		}
	}

	pub fn world_pos(&self, pos: UVec2) -> Vec3 {
		self.offset
			+ Vec3::new(
				pos.x as f32 * self.cell_width,
				0.,
				pos.y as f32 * self.cell_width,
			)
	}
}
