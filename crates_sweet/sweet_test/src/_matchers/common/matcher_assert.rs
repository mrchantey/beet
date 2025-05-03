//! These are assertions commonly used by other matchers
//! by convention, any matcher function beginning with 'assert'
//! is used internally, and should only be called at the top
//! level of the matcher
use crate::prelude::*;
use std::fmt::Debug;


impl<T> Matcher<T> {
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	#[allow(unused)]
	pub(crate) fn assert_some_with_received<T2>(&self, received: Option<T2>) {
		self.panic_if_negated();
		if received.is_none() {
			self.panic_with_expected_received(&"Some", &"None");
		}
	}

	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	#[allow(unused)]
	pub(crate) fn assert_option_with_received_negatable<T2>(
		&self,
		received: Option<T2>,
	) {
		if self.negated && received.is_some() {
			self.panic_with_expected_received(&"Some", &"Some");
		} else if !self.negated && received.is_none() {
			self.panic_with_expected_received(&"Some", &"None");
		}
	}

	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	pub(crate) fn assert_correct_with_received<T2: Debug, T3: Debug>(
		&self,
		result: bool,
		expected: &T2,
		received: &T3,
	) {
		if !self.is_true_with_negated(result) {
			self.panic_with_expected_received(expected, received)
		}
	}

	/// Always panics, level 3 wrapper for [Self::panic_with_expected_received]
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	pub(crate) fn assert_with_expected_received<T2: Debug, T3: Debug>(
		&self,
		expected: &T2,
		received: &T3,
	) -> ! {
		self.panic_with_expected_received(expected, received)
	}
}


impl<T: Debug> Matcher<T> {
	/// Ensure result is true, and check negated
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	pub fn assert_correct<T2: Debug>(&self, result: bool, expected: &T2) {
		if !self.is_true_with_negated(result) {
			self.panic_with_expected_received(expected, &self.value)
		}
	}
}


impl<T: Debug> Matcher<T> {
	/// Ensure results are equal, and check negated
	/// Must be called at [`SweetError::BACKTRACE_LEVEL_3`]
	pub(crate) fn assert_equal<T2: Debug>(&self, expected: &T2)
	where
		T: PartialEq<T2>,
	{
		if !self.is_true_with_negated((&self.value) == expected) {
			self.panic_with_expected_received(expected, &self.value)
		}
	}
}
