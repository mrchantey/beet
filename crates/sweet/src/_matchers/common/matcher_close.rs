use super::*;
use extend::ext;
use std::fmt::Debug;


#[extend::ext(name=SweetClose)]
pub impl<T: CloseTo + Copy + Debug> T {
	/// Asserts that the value is close to `expected`,
	/// using [`CloseTo::default_delta`] of `0.1` as the allowed difference.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 0.0.xpect_close(0.01);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not within the default delta of `expected`.
	fn xpect_close(&self, expected: impl Into<T>) {
		let delta = T::default_delta();
		let expected = expected.into();
		if !self.is_close_with_delta(&expected, &delta) {
			let expected =
				format!("close to {:?} within {:?}", expected, delta);
			assert_ext::panic_expected_received_display_debug(&expected, self);
		}
	}
	/// Asserts that the value is close to `expected`,
	/// using the provided delta as the allowed difference.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 3.14_f32.xpect_close_within(3.1415, 0.01_f32);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not within the given `delta` of `expected`.
	fn xpect_close_within(&self, expected: impl Into<T>, delta: impl Into<T>) {
		let delta = delta.into();
		let expected = expected.into();
		if !self.is_close_with_delta(&expected, &delta) {
			let expected =
				format!("close to {:?} within {:?}", expected, delta);
			assert_ext::panic_expected_received_display_debug(&expected, self);
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
	use bevy::prelude::Deref;

	#[derive(Clone, Deref)]
	struct NewType<T>(pub T);

	#[test]
	fn to_be_close_to() {
		(0.0_f64).xpect_close(0.);
		(-0.999_f64).xpect_close(-1.);
		NewType(0.0_f64).xpect_close(0.);
		0.0_f64.xpect_close_within(0.5, 1.);

		(0.0_f32).xpect_close(0.);
		(-0.999_f32).xpect_close(-1.);
		NewType(0.0_f32).xpect_close(0.);
		0.0_f32.xpect_close_within(0.5, 1.);
	}
}
