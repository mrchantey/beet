use bevy::prelude::*;
use std::f32::consts::TAU;

/// Arrange children in a circle around the parent
pub fn orbital_child(index: usize, total: usize) -> Transform {
	let angle = TAU / total as f32 * index as f32;
	let pos = Vec3::new(f32::cos(angle), f32::sin(angle), 0.);
	Transform::from_translation(pos * 0.7).with_scale(CHILD_SCALE)
}


const CHILD_SCALE: Vec3 = Vec3 {
	x: 0.5,
	y: 0.5,
	z: 0.5,
};
