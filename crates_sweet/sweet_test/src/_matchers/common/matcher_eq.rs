use super::*;
use std::fmt::Debug;

impl<T> Matcher<T>
where
	T: Debug,
{
	/// Assert that the received value is equal to the expected value.
	/// # Example
	/// ```rust
	/// # use sweet_test::prelude::*;
	/// expect(7).to_be(7);
	/// expect("foo").not().to_be("bar");
	/// ```
	pub fn to_be<T2: Debug>(&self, other: T2) -> &Self
	where
		T: PartialEq<T2>,
	{
		self.assert_equal(&other);
		self
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn equality() {
		expect(true).to_be(true);
		expect(false).to_be(false);
		expect(true).not().to_be(false);
	}
}
