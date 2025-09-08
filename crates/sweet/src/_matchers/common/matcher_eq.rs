use super::*;
use std::fmt::Debug;

impl<T> Matcher<T>
where
	T: Debug,
{
	/// Assert that the received value is equal to the expected value.
	/// # Example
	/// ```rust
	/// # use sweet::prelude::*;
	/// 7.xpect().to_be(7);
	/// "foo".xpect().not().to_be("bar");
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
		true.xpect().to_be(true);
		false.xpect().to_be(false);
		true.xpect().not().to_be(false);
	}
}
