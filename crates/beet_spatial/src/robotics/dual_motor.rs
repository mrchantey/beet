use crate::prelude::*;
use beet_core::prelude::*;



/// A handy abstraction over two motor values,
/// one for the left motor and one for the right motor.
#[derive(Default, Debug, Copy, Clone, PartialEq, Component, Reflect)]
pub struct DualMotorValue {
	/// The value for the left motor
	pub left: MotorValue,
	/// The value for the right motor
	pub right: MotorValue,
}

impl DualMotorValue {
	/// Create a new dual motor value with the given left and right motor values
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
	/// Apply the given value to both motors
	pub fn splat(value: MotorValue) -> Self { Self::new(value, value) }
}
