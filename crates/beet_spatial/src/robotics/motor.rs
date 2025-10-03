use beet_core::prelude::*;
use strum_macros::Display;
use strum_macros::EnumIter;

/// Specifies the direction a motor should run in
#[derive(
	Debug, Default, Copy, Clone, PartialEq, Reflect, EnumIter, Display,
)]
pub enum MotorDirection {
	/// The motor should run forward
	#[default]
	Forward,
	/// The motor should run backward
	Backward,
}

/// Represents the current value of a motor, split into speed and direction
#[derive(Debug, Default, Copy, Clone, PartialEq, Reflect, Component)]
pub struct MotorValue {
	/// The speed at which the motor should run, 0 is stopped and 255 is full speed
	pub speed: u8,
	/// The direction the motor should run in
	pub direction: MotorDirection,
}

impl MotorValue {
	/// Create a new motor value with the given speed and direction
	pub fn new(speed: u8, direction: MotorDirection) -> Self {
		Self { speed, direction }
	}
	/// Returns a value between -1 (backward) and 1 (forward)
	pub fn to_signed_normal(&self) -> f32 {
		let normalized = self.speed as f32 / u8::MAX as f32;
		match self.direction {
			MotorDirection::Forward => normalized,
			MotorDirection::Backward => -normalized,
		}
	}

	/// apply from a value between -1 and 1
	/// # Panics
	/// Panics if the value is not between -1 and 1
	pub fn from_signed_normal(normal: f32) -> Self {
		let value = (normal.abs() * u8::MAX as f32) as u8;
		let direction = if normal >= 0.0 {
			MotorDirection::Forward
		} else {
			MotorDirection::Backward
		};
		Self::new(value, direction)
	}
	/// Create a new motor value that stops the motor
	pub fn stop() -> Self { Self::new(0, MotorDirection::Forward) }
	/// Create a new motor value that runs forward at the given speed
	pub fn forward(speed: u8) -> Self {
		Self::new(speed, MotorDirection::Forward)
	}
	/// Create a new motor value that runs backward at the given speed
	pub fn backward(speed: u8) -> Self {
		Self::new(speed, MotorDirection::Backward)
	}
	/// Create a new motor value that runs forward at full speed
	pub fn forward_max() -> Self { Self::forward(u8::MAX) }
	/// Create a new motor value that runs backward at full speed
	pub fn backward_max() -> Self { Self::backward(u8::MAX) }
}
