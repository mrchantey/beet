use bevy::prelude::*;
use std::f32::consts::TAU;


/// Enum of common curves that can be serialized
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Debug, Default)]
pub enum SerdeCurve {
	#[default]
	Circle,
	Square,
	EaseDir2(EasingCurve<Dir2>),
}

impl Curve<Vec3> for SerdeCurve {
	fn domain(&self) -> Interval { Interval::EVERYWHERE }

	fn sample_unchecked(&self, t: f32) -> Vec3 {
		match self {
			SerdeCurve::Circle => circle_curve(t),
			SerdeCurve::Square => square_curve(t),
			SerdeCurve::EaseDir2(ease) => ease.sample_unchecked(t).extend(0.),
		}
	}
}

impl Into<SerdeCurve> for EasingCurve<Dir2> {
	fn into(self) -> SerdeCurve { SerdeCurve::EaseDir2(self) }
}


fn circle_curve(t: f32) -> Vec3 {
	let angle = t * TAU;
	Vec3::new(angle.cos(), angle.sin(), 0.)
}

fn square_curve(t: f32) -> Vec3 {
	let t = t * 4.;
	let x = if t < 1. {
		1.
	} else if t < 2. {
		1. - (t - 1.)
	} else if t < 3. {
		-1.
	} else {
		-1. + (t - 3.)
	};
	let y = if t < 1. {
		t
	} else if t < 2. {
		1.
	} else if t < 3. {
		1. - (t - 2.)
	} else {
		-1.
	};
	Vec3::new(x, y, 0.)
}
