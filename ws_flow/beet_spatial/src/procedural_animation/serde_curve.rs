use bevy::prelude::*;
use std::f32::consts::TAU;


/// Enum of common curves that can be serialized.
/// This will likely be deprecated when bevy_curve gets better support for serde.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Debug, Default)]
pub enum SerdeCurve {
	/// Animate along a unit circle in xy space, with the z component set to 0.
	#[default]
	Circle,
	/// Animate along a unit square in xy space, with the z component set to 0.
	Square,
	/// Easing curve in xy space, with the z component set to 0.
	EaseDir2(EasingCurve<Dir2>),
	/// Easing curve in xyz space.
	EaseVec3(EasingCurve<Vec3>),
	/// A curve defined by a set of samples.
	Samples(SampleAutoCurve<Vec3>),
}


const DEFAULT_TOTAL_LEN_SAMPLES: usize = 32;

impl SerdeCurve {
	/// Calculate the total length of the curve with a small number of samples,
	/// usually providing accuracy within 10%.
	/// See [`Self::total_len_with_samples`] for passing a higher number of samples.
	pub fn total_len(&self) -> f32 {
		self.total_len_with_samples(DEFAULT_TOTAL_LEN_SAMPLES)
	}


	/// Calculate the total length of the curve by sampling it at
	/// regular intervals. The number of samples determines the accuracy
	/// and the returned length is <= the actual length.
	pub fn total_len_with_samples(&self, num_samples: usize) -> f32 {
		let mut total_len = 0.;
		let delta_t = 1.0 / (num_samples as f32);
		let mut last_pos = self.sample_unchecked(0.);
		for i in 1..num_samples {
			let t = i as f32 * delta_t;
			let pos = self.sample_unchecked(t);
			total_len += pos.distance(last_pos);
			last_pos = pos;
		}
		total_len
	}
}

impl Curve<Vec3> for SerdeCurve {
	fn domain(&self) -> Interval { Interval::EVERYWHERE }

	fn sample_unchecked(&self, t: f32) -> Vec3 {
		match self {
			Self::Circle => circle_curve(t),
			Self::Square => square_curve(t),
			Self::EaseDir2(ease) => ease.sample_unchecked(t).extend(0.),
			Self::EaseVec3(ease) => ease.sample_unchecked(t),
			Self::Samples(samples) => samples.sample_unchecked(t),
		}
	}
}

impl Into<SerdeCurve> for EasingCurve<Dir2> {
	fn into(self) -> SerdeCurve { SerdeCurve::EaseDir2(self) }
}
impl Into<SerdeCurve> for EasingCurve<Vec3> {
	fn into(self) -> SerdeCurve { SerdeCurve::EaseVec3(self) }
}

impl Into<SerdeCurve> for SampleAutoCurve<Vec3> {
	fn into(self) -> SerdeCurve { SerdeCurve::Samples(self) }
}


fn circle_curve(t: f32) -> Vec3 {
	let angle = t * TAU;
	Vec3::new(angle.cos(), angle.sin(), 0.)
}

fn square_curve(t: f32) -> Vec3 {
	if t < 0.25 {
		Vec3::new(1., 4. * t, 0.)
	} else if t < 0.5 {
		Vec3::new(1. - 4. * (t - 0.25), 1., 0.)
	} else if t < 0.75 {
		Vec3::new(0., 1. - 4. * (t - 0.5), 0.)
	} else {
		Vec3::new(4. * (t - 0.75), 0., 0.)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use std::f32::consts::PI;
	use std::f32::consts::TAU;
	use sweet::prelude::*;

	#[test]
	fn calculates_length() {
		expect(SerdeCurve::Circle.total_len()).to_be_less_than(TAU);
		expect(SerdeCurve::Circle.total_len()).to_be_greater_than(6.);
		expect(SerdeCurve::Square.total_len()).to_be_less_than(4.);
		expect(SerdeCurve::Square.total_len()).to_be_greater_than(3.8);


		let ease = SerdeCurve::EaseDir2(EasingCurve::new(
			Dir2::X,
			Dir2::Y,
			EaseFunction::CubicInOut,
		));
		expect(ease.total_len()).to_be_less_than(PI);
		expect(ease.total_len()).to_be_greater_than(1.5);
	}
}
