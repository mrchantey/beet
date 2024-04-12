use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

#[derive(
	Default,
	Debug,
	Copy,
	Clone,
	PartialEq,
	Component,
	Serialize,
	Deserialize,
	FieldUi,
)]
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