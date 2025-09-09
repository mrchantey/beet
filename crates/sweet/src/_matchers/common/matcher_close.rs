use super::*;
use extend::ext;
use std::fmt::Debug;
use std::fmt::Display;


#[extend::ext(name=SweetClose)]
pub impl<T, U> T
where
	T: IntoMaybeNot<U>,
	U: CloseTo + Display,
{
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
	fn xpect_close(self, expected: U) {
		let delta = U::default_delta();
		let received = self.into_maybe_not();
		assert(expected, received, delta);
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
	fn xpect_close_within(self, expected: U, delta: U) {
		let received = self.into_maybe_not();
		assert(expected, received, delta);
	}
}
fn assert<T: CloseTo + Display>(expected: T, received: MaybeNot<T>, delta: T) {
	let result = received.inner().is_close_with_delta(&expected, &delta);
	let expected = format!("within {} of {}", delta, expected,);
	if let Err(expected) = received.passes_with_message(result, expected) {
		panic_ext::panic_expected_received_display(
			&expected,
			&received.inner(),
		);
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
		(0.0_f64).xnot().xpect_close(10.);
		(0.0_f64).xpect_close(0.01);
		(-0.999_f64).xpect_close(-1.);
		NewType(0.0_f64).xpect_close(0.);
		0.0_f64.xpect_close_within(0.5, 1.);
	}
}
