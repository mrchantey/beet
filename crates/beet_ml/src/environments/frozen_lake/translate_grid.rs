use crate::prelude::ActionSpace;
use beet_ecs::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
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
	pub fn sat_add(&mut self, other: IVec2) {
		fn add(u: u32, i: i32) -> u32 {
			if i.is_negative() {
				u - i.wrapping_abs() as u32
			} else {
				u + i as u32
			}
		}
		self.0.x = add(self.0.x, other.x);
		self.0.y = add(self.0.y, other.y);
	}
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

	pub fn with_adjacents(&self) -> Vec<Self> {
		match self {
			Self::Up => vec![Self::Left, Self::Up, Self::Right],
			Self::Right => vec![Self::Up, Self::Right, Self::Down],
			Self::Down => vec![Self::Right, Self::Down, Self::Left],
			Self::Left => vec![Self::Down, Self::Left, Self::Up],
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

#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Default, Component, ActionMeta)]
pub struct TranslateGrid {
	pub speed: f32,
}

impl Default for TranslateGrid {
	fn default() -> Self { Self { speed: 1.0 } }
}

fn translate_grid(
	mut commands: Commands,
	mut agents: Query<(&mut Transform, &mut GridPos, &GridDirection)>,
	action: Query<(Entity, &TranslateGrid, &TargetAgent), With<Running>>,
) {
	for (entity, _, agent) in action.iter() {
		let Ok((mut transform, mut pos, dir)) = agents.get_mut(**agent) else {
			continue;
		};
		pos.sat_add((*dir).into());
		transform.translation = Vec3::new(pos.0.x as f32, 0.0, pos.0.y as f32);
		// TODO incremental move and check if done
		commands.entity(entity).insert(RunResult::Success);
	}
}

impl ActionMeta for TranslateGrid {
	fn category(&self) -> ActionCategory { ActionCategory::Behavior }
}

impl ActionSystems for TranslateGrid {
	fn systems() -> SystemConfigs { translate_grid.in_set(TickSet) }
}
