use std::fmt::Display;

use super::*;

#[extend::ext(name=SweetString)]
pub impl<T, U> T
where
	T: IntoMaybeNot<U>,
	U: AsRef<str> + Display,
{
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
}

impl<T: std::fmt::Debug + AsRef<str>> Matcher<T> {
	pub fn to_contain(&self, other: impl AsRef<str>) -> &Self {
		let other = other.as_ref();
		let result = self.value.as_ref().contains(other);
		let expected = format!("to contain '{}'", other);
		self.assert_correct(result, &expected);
		self
	}
	pub fn to_contain_n(&self, other: impl AsRef<str>, count: usize) -> &Self {
		let other = other.as_ref();
		let actual_count = self.value.as_ref().matches(other).count();
		let result = actual_count == count;
		let expected = format!("{count} occurances of '{}'", other);
		self.assert_correct_with_received(
			result,
			&expected,
			&format!("{actual_count} occurances\n{}", self.value.as_ref()),
		);
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
	fn str() {
		"foobar".xpect_contains("bar");
		// "foobar".xpect_contains("barss");
		// "foobar".xnot().xpect_contains("bazz");


		"foobar".xpect().not().to_contain("baz");

		"foobar".xpect().to_start_with("foo");
		"foobar".xpect().not().to_start_with("bar");

		"foobar".xpect().to_end_with("bar");
		"foobar".xpect().not().to_end_with("foo");
	}
}
