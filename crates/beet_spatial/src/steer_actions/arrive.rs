use crate::prelude::*;
use beet_core::prelude::*;

/// The distance at which an agent should begin to slow down, defaults to `0.5`
#[derive(
	Debug, Copy, Clone, PartialEq, Deref, DerefMut, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct ArriveRadius(pub f32);

impl Default for ArriveRadius {
	fn default() -> Self { Self(0.7) }
}

/// Calculates an arrive speed
/// as described [here](https://youtu.be/OxHJ-o_bbzs?list=PLRqwX-V7Uu6ZV4yEcW3uDwOgGXKUUsPOM&t=439)
pub fn arrive_speed(
	position: &Vec3,
	target_position: &Vec3,
	max_speed: MaxSpeed,
	arrive_radius: Option<ArriveRadius>,
) -> MaxSpeed {
	if let Some(arrive_radius) = arrive_radius {
		let distance = (*target_position - *position).length();
		if distance < *arrive_radius {
			MaxSpeed(f32::lerp(0.0, max_speed.0, distance / *arrive_radius))
		} else {
			max_speed
		}
	} else {
		max_speed
	}
}
