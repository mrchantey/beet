use std::fmt::Display;

use super::*;

#[extend::ext(name=SweetString)]
pub impl<T, U> T
where
	T: IntoMaybeNotDisplay<U>,
	U: AsRef<str> + Display,
{
	fn xpect_str(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		assert_ext::assert_diff(expected, received).into_inner()
	}
	fn xpect_contains(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().contains(expected);
		let expected = format!("to contain '{}'", expected);
		assert_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}

	fn xpect_starts_with(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().starts_with(expected);
		let expected = format!("to start with '{}'", expected);
		assert_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}

	fn xpect_ends_with(self, expected: impl AsRef<str>) -> U {
		let expected = expected.as_ref();
		let received = self.into_maybe_not();
		let result = received.inner().as_ref().ends_with(expected);
		let expected = format!("to end with '{}'", expected);
		assert_ext::assert_result_expected_received_display(
			result, expected, received,
		)
		.into_inner()
	}
}

impl<T: std::fmt::Debug + AsRef<str>> Matcher<T> {
	pub fn to_contain(&self, other: impl AsRef<str>) -> &Self {
		let other = other.as_ref();
		let result = self.value.as_ref().contains(other);
		let expected = format!("to contain '{}'", other);
		self.assert_correct(result, &expected);
		self
	}

	pub fn to_start_with(&self, other: impl AsRef<str>) -> &Self {
		let other = other.as_ref();
		let result = self.value.as_ref().starts_with(other);
		let expected = format!("to start with '{}'", other);
		self.assert_correct(result, &expected);
		self
	}
	pub fn to_end_with(&self, other: impl AsRef<str>) -> &Self {
		let other = other.as_ref();
		let result = self.value.as_ref().ends_with(other);
		let expected = format!("to end with '{}'", other);
		self.assert_correct(result, &expected);
		self
	}
	/// Like `to_be`, but with pretty diffing
	pub fn to_be_str(&self, other: impl AsRef<str>) -> &Self {
		self.assert_diff(other.as_ref(), self.value.as_ref());
		self
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

		"foobar".xpect().not().to_contain("baz");

		"foobar".xpect_starts_with("foo");
		"foobar".xnot().xpect_starts_with("bar");

		"foobar".xpect_ends_with("bar");
		"foobar".xnot().xpect_ends_with("foo");
	}
}
