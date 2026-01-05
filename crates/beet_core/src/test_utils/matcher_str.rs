use std::fmt::Display;

use super::*;

#[extend::ext(name=SweetString)]
pub impl<T, U> T
where
	T: IntoMaybeNotDisplay<U>,
	U: AsRef<str> + Display,
{
	/// Performs an assertion ensuring this string is equal to `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// "foo".xpect_str("foo");
	/// "foo".xnot().xpect_str("bar");
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not equal to `expected`.
	#[track_caller]
	fn xpect_str(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		panic_ext::assert_diff(expected, received).into_inner()
	}

	/// Performs an assertion ensuring this string contains `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// "foobar".xpect_contains("bar");
	/// "foobar".xnot().xpect_contains("bazz");
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value does not contain `expected`.
	#[track_caller]
	fn xpect_contains(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().contains(expected);
		let expected = format!("to contain '{}'", expected);
		panic_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}

	/// Performs an assertion ensuring this string starts with `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// "foobar".xpect_starts_with("foo");
	/// "foobar".xnot().xpect_starts_with("bazz");
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value does not start with `expected`.
	#[track_caller]
	fn xpect_starts_with(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().starts_with(expected);
		let expected = format!("to start with '{}'", expected);
		panic_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}

	/// Performs an assertion ensuring this string ends with `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// "foobar".xpect_ends_with("bar");
	/// "foobar".xnot().xpect_ends_with("bazz");
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value does not end with `expected`.
	#[track_caller]
	fn xpect_ends_with(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().ends_with(expected);
		let expected = format!("to end with '{}'", expected);
		panic_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	#[should_panic]
	fn err_xpect_contains() { "foobar".xpect_contains("bazz"); }
	#[test]
	#[should_panic]
	fn err_xpect_str() { "foobar".xpect_str("bazz"); }
	#[test]
	#[should_panic]
	fn err_xpect_not_str() { "foobar".xnot().xpect_str("foobar"); }

	#[test]
	fn str() {
		"foobar".xpect_contains("bar");
		"foobar".xnot().xpect_contains("bazz");

		"foobar".xnot().xpect_str("bazz");
		"foobar".xpect_str("foobar");

		"foobar".xpect_starts_with("foo");
		"foobar".xnot().xpect_starts_with("bar");

		"foobar".xpect_ends_with("bar");
		"foobar".xnot().xpect_ends_with("foo");
	}
}
