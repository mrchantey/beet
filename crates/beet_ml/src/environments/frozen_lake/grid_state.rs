use bevy::prelude::*;

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, Deref, DerefMut, Reflect)]
pub struct GridPos(pub UVec2);

impl GridPos {
	pub fn new(pos: UVec2) -> Self { Self(pos) }
}

impl From<UVec2> for GridPos {
	fn from(pos: UVec2) -> Self { Self(pos) }
}


// impl DiscreteSpace for GridPos {
// 	// const LEN: usize = WIDTH * HEIGHT;
// }
