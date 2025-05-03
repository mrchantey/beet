use super::*;
use extend::ext;
use std::fmt::Debug;

#[ext(name=MatcherExtClose)]
/// Matcher Extensions for types that implement `CloseTo`: `f32`, `f64`, `Vec3`, etc.
pub impl<T: CloseTo + Copy + Debug> Matcher<T>
// where
// U: CloseTo + std::fmt::Debug + Copy,
{
	fn to_be_close_to(&self, expected: impl Into<T>) {
		let received = self.value;
		let expected = expected.into();
		let result = T::is_close(received, expected);
		let expected = format!("close to {:?}", expected);
		self.assert_correct_with_received(result, &expected, &received);
	}
	fn to_be_close_to_with_epsilon(&self, expected: impl Into<T>, epsilon: T) {
		let received = self.value;
		let expected = expected.into();
		let result = T::is_close_with_epsilon(received, expected, epsilon);
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
		expect(0.).to_be_close_to(0.);
		expect(-0.999).to_be_close_to(-1.);
		expect(0.9).not().to_be_close_to(1.01);
		expect(NewType(0.0_f64).0).to_be_close_to(0.);

		expect(0.0_f32).to_be_close_to(0.);
		expect(NewType(0.0_f32).0).to_be_close_to(0.);
	}
}
