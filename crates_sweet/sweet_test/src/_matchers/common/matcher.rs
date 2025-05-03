//TODO should probably be in matcher module

/// The base struct for all matchers.
pub struct Matcher<T> {
	pub value: T,
	pub negated: bool,
}


impl<T> Matcher<T> {
	/// Create a new Matcher, usually this is done via [`expect`](crate::expect).
	pub(crate) fn new(value: T) -> Matcher<T> {
		Matcher {
			value,
			negated: false,
		}
	}

	/// Map the value of this matcher to a new matcher with the mapped value.
	pub fn map<T2>(&self, func: impl FnOnce(&T) -> T2) -> Matcher<T2> {
		Matcher::new(func(&self.value))
	}
	/// Negate this matcher to flip the result of an assertion.
	/// ```rust
	/// # use sweet_test::prelude::*;
	/// expect(true).not().to_be_false();
	/// ```
	pub fn not(&mut self) -> &mut Self {
		self.negated = true;
		self
	}

	/// Return a boolean as-is if not negated, otherwise flip it.
	pub fn is_true_with_negated(&self, received: bool) -> bool {
		if self.negated {
			!received
		} else {
			received
		}
	}
}
