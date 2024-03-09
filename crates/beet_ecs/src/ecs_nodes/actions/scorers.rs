use crate::prelude::*;
use bevy_ecs::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[action(system=empty_action)]
#[derive(Default)]
pub struct ConstantScore {
	#[shared]
	pub score: Score,
}

impl ConstantScore {
	pub fn new(score: Score) -> Self { Self { score } }
}
