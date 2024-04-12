use crate::prelude::*;
use bevy::prelude::*;

#[derive(Default, Debug, Copy, Clone, PartialEq, Component, Reflect)]
pub struct DualMotorValue {
	pub left: MotorValue,
	pub right: MotorValue,
}

impl DualMotorValue {
	pub fn new(left: MotorValue, right: MotorValue) -> Self {
		Self { left, right }
	}
	/// Set direction from a Vec2,  +y = forward. Will normalize for you.
	pub fn new_from_dir(dir: Vec2) -> Self {
		let dir = dir.normalize();
		Self {
			left: MotorValue::from_signed_normal(dir.y + dir.x),
			right: MotorValue::from_signed_normal(dir.y - dir.x),
		}
	}

	pub fn splat(value: MotorValue) -> Self { Self::new(value, value) }
}
