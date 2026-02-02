//! Negation wrapper for matcher assertions.
//!
//! This module provides [`MaybeNot`], a wrapper type that allows negating
//! assertions. When wrapped with `xnot()`, assertions check for the opposite
//! condition.
//!
//! # Example
//!
//! ```
//! # use beet_core::prelude::*;
//! // MaybeNot is an internal wrapper used by matchers
//! let value = MaybeNot::Negated(5);
//! value.passes(&10).xpect_true(); // passes because 5 != 10
//! value.passes(&5).xpect_false(); // fails because negated
//! ```
use std::fmt::Display;

/// Extension trait adding the `xnot()` method to all types.
#[extend::ext(name=MatcherNot)]
pub impl<T> T {
	/// Negates an outcome, wrapping it in [`MaybeNot::Negated`].
	fn xnot(self) -> MaybeNot<T> { MaybeNot::Negated(self) }
}

/// A wrapper that tracks whether a value should be negated in assertions.
#[derive(Debug, Copy, Clone)]
pub enum MaybeNot<T> {
	/// The value's assertions should be negated.
	Negated(T),
	/// The value's assertions should be applied verbatim.
	Verbatim(T),
}

impl<T> MaybeNot<T> {
	/// Returns `true` if this is a negated value.
	pub fn is_negated(&self) -> bool {
		match self {
			MaybeNot::Negated(_) => true,
			MaybeNot::Verbatim(_) => false,
		}
	}

	/// Returns a reference to the inner value.
	pub fn inner(&self) -> &T {
		match self {
			MaybeNot::Negated(value) => value,
			MaybeNot::Verbatim(value) => value,
		}
	}

	/// Consumes the wrapper and returns the inner value.
	pub fn into_inner(self) -> T {
		match self {
			MaybeNot::Negated(value) => value,
			MaybeNot::Verbatim(value) => value,
		}
	}

	fn format_expected(&self, expected: String) -> String {
		match self.is_negated() {
			true => format!("NOT {}", expected),
			false => expected,
		}
	}

	/// Checks if the result passes, considering negation, and formats using Display.
	pub fn passes_display(
		&self,
		result: bool,
		expected: impl std::fmt::Display,
	) -> Result<(), String> {
		match (result, self.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			_ => Err(self.format_expected(format!("{}", expected))),
		}
	}

	/// Checks if the result passes, considering negation, and formats using Debug.
	pub fn passes_debug(
		&self,
		result: bool,
		expected: impl std::fmt::Debug,
	) -> Result<(), String> {
		match (result, self.is_negated()) {
			(true, false) => Ok(()),
			(false, true) => Ok(()),
			_ => Err(self.format_expected(format!("{:?}", expected))),
		}
	}

	/// Performs an equality check, considering if `self` is negated.
	pub fn passes<T2>(&self, other: &T2) -> bool
	where
		T: PartialEq<T2>,
	{
		match self {
			MaybeNot::Negated(value) => value != other,
			MaybeNot::Verbatim(value) => value == other,
		}
	}

	/// Compares with expected value, formatting errors with Display.
	pub fn compare_display<Expected>(
		&self,
		expected: &Expected,
	) -> Result<(), String>
	where
		Expected: std::fmt::Display,
		T: PartialEq<Expected>,
	{
		if self.passes(expected) {
			Ok(())
		} else {
			Err(self.format_expected(format!("{}", expected)))
		}
	}

	/// Compares with expected value, formatting errors with Debug.
	pub fn compare_debug<Expected>(
		&self,
		expected: &Expected,
	) -> Result<(), String>
	where
		Expected: std::fmt::Debug,
		T: PartialEq<Expected>,
	{
		if self.passes(expected) {
			Ok(())
		} else {
			Err(self.format_expected(format!("{:?}", expected)))
		}
	}
}

/// Trait for converting values into [`MaybeNot`] wrappers.
///
/// This blanket trait is implemented for:
/// - All `Display` types (wrapped as `Verbatim`)
/// - `MaybeNot<T>` where `T: Display` (passed through)
///
/// This allows assertions to accept both raw values and negated values.
pub trait IntoMaybeNotDisplay<T>: Sized {
	/// Converts this value into a [`MaybeNot`] wrapper.
	fn into_maybe_not(self) -> MaybeNot<T>;
}

impl<T> IntoMaybeNotDisplay<T> for T
where
	T: Display,
{
	fn into_maybe_not(self) -> MaybeNot<T> { MaybeNot::Verbatim(self) }
}

impl<T> IntoMaybeNotDisplay<T> for MaybeNot<T>
where
	T: Display,
{
	fn into_maybe_not(self) -> MaybeNot<T> { self }
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[extend::ext]
	impl<T, U> T
	where
		Self: IntoMaybeNotDisplay<U>,
		U: PartialEq + std::fmt::Debug,
	{
		fn check(self, expected: U) -> Result<(), String> {
			self.into_maybe_not().compare_debug(&expected)
		}
		// this only works because of MaybeNotDisplay,
		// otherwise multiple impls
		fn check_untyped<V>(self, expected: V) -> Result<(), String>
		where
			V: std::fmt::Debug,
			U: PartialEq<V>,
		{
			self.into_maybe_not().compare_debug(&expected)
		}
	}

	#[test]
	fn test_check() {
		true.check(true).xpect_ok();
		true.check(false).xpect_err();
		true.xnot().check(false).xpect_ok();
		true.xnot().check(true).xpect_err();

		MaybeNot::Negated(false).check(true).xpect_ok();
		MaybeNot::Negated(false).check_untyped(true).xpect_ok();
	}

	#[test]
	fn passes() {
		MaybeNot::Verbatim(true).passes(&true).xpect_true();
		MaybeNot::Verbatim(false).passes(&true).xpect_false();
	}

	#[test]
	fn into() {
		let _val: MaybeNot<_> = true.into_maybe_not();
		let _val: MaybeNot<bool> = MaybeNot::Negated(true).into();
	}
}
