use super::*;
use extend::ext;
use std::fmt::Debug;


#[extend::ext(name=SweetClose)]
pub impl<T: CloseTo + Copy + Debug> T {
	fn xpect_close(&self, expected: impl Into<T>) {
		let delta = T::default_delta();
		let expected = expected.into();
		if !self.is_close_with_delta(&expected, &delta) {
			let expected =
				format!("close to {:?} within {:?}", expected, delta);
			assert_ext::panic_expected_received_debug(&expected, self);
		}
	}
	fn xpect_close_within(&self, expected: impl Into<T>, delta: impl Into<T>) {
		let delta = delta.into();
		let expected = expected.into();
		if !self.is_close_with_delta(&expected, &delta) {
			let expected =
				format!("close to {:?} within {:?}", expected, delta);
			assert_ext::panic_expected_received_debug(&expected, self);
		}
	}
}


#[ext(name=MatcherExtClose)]
/// Matcher Extensions for types that implement `CloseTo`: `f32`, `f64`, `Vec3`, etc.
pub impl<T: CloseTo + Copy + Debug> Matcher<T>
// where
// U: CloseTo + std::fmt::Debug + Copy,
{
	fn to_be_close_to(&self, expected: impl Into<T>) {
		let received = self.value;
		let expected = expected.into();
		let result = T::is_close(&received, &expected);
		let expected = format!("close to {:?}", expected);
		self.assert_correct_with_received(result, &expected, &received);
	}
	fn to_be_close_to_with_epsilon(&self, expected: impl Into<T>, epsilon: T) {
		let received = self.value;
		let expected = expected.into();
		let result = T::is_close_with_delta(&received, &expected, &epsilon);
		let expected = format!("close to {:?}", expected);
		self.assert_correct_with_received(result, &expected, &received);
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[derive(Clone)]
	struct NewType<T>(pub T);

	#[test]
	fn to_be_close_to() {
		(0.).xpect().to_be_close_to(0.);
		(-0.999).xpect().to_be_close_to(-1.);
		(0.9).xpect().not().to_be_close_to(1.01);
		NewType(0.0_f64).0.xpect().to_be_close_to(0.);

		0.0_f32.xpect().to_be_close_to(0.);
		NewType(0.0_f32).0.xpect().to_be_close_to(0.);
	}
}
