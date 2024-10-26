use bevy::prelude::*;


/// Enum of common curves that can be serialized
#[derive(Debug, Clone, PartialEq, Component, Reflect)]
#[reflect(Debug, Default, PartialEq)]
pub enum SerdeCurve {
	Circle,
	Square,
}

impl Default for SerdeCurve {
	fn default() -> Self { Self::Circle }
}


impl Curve<Vec3> for SerdeCurve {
	fn domain(&self) -> Interval { Interval::EVERYWHERE }

	fn sample_unchecked(&self, t: f32) -> Vec3 {
		match self {
			SerdeCurve::Circle => circle_curve(t),
			SerdeCurve::Square => square_curve(t),
		}
	}
}




fn circle_curve(t: f32) -> Vec3 { Vec3::new(t.cos(), t.sin(), 0.) }

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
