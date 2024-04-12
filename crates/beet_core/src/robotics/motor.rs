use beet_ecs::prelude::*;
use bevy_ecs::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use strum_macros::Display;
use strum_macros::EnumIter;

#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	Serialize,
	Deserialize,
	FieldUi,
	EnumIter,
	Display,
)]
pub enum MotorDirection {
	#[default]
	Forward,
	Backward,
}

#[derive(
	Debug,
	Default,
	Copy,
	Clone,
	PartialEq,
	Serialize,
	Deserialize,
	FieldUi,
	Component,
)]
pub struct MotorValue {
	pub value: u8,
	pub direction: MotorDirection,
}

impl MotorValue {
	pub fn new(value: u8, direction: MotorDirection) -> Self {
		Self { value, direction }
	}
	/// Returns a value between -1 (backward) and 1 (forward)
	pub fn to_signed_normal(&self) -> f32 {
		let normalized = self.value as f32 / u8::MAX as f32;
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

	pub fn stop() -> Self { Self::new(0, MotorDirection::Forward) }
	pub fn forward(value: u8) -> Self {
		Self::new(value, MotorDirection::Forward)
	}
	pub fn backward(value: u8) -> Self {
		Self::new(value, MotorDirection::Backward)
	}
	pub fn forward_max() -> Self { Self::forward(u8::MAX) }
	pub fn backward_max() -> Self { Self::backward(u8::MAX) }
}
