use super::*;
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


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::Deref;

	#[derive(Clone, Deref)]
	struct NewType<T>(pub T);

	#[test]
	fn close_to_behavior() {
		(0.0_f64).xpect_close(0.);
		(0.0_f64).xnot().xpect_close(10.);
		(0.0_f64).xpect_close(0.01);
		(-0.999_f64).xpect_close(-1.);
		NewType(0.0_f64).xpect_close(0.);
		0.0_f64.xpect_close_within(0.5, 1.);
	}
}
