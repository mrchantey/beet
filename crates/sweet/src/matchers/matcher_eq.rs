use super::*;
use std::fmt::Debug;

#[extend::ext(name=SweetEq)]
pub impl<T> T
where
	T: Debug,
{
	/// Performs an assertion ensuring this value is equal to `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 1.xpect_eq(1);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not equal to `expected`.
	fn xpect_eq<U>(&self, expected: U) -> &Self
	where
		T: PartialEq<U>,
		U: Debug,
	{
		if self != &expected {
			assert_ext::panic_expected_received_debug(expected, self);
		}
		self
	}
	/// Performs an assertion ensuring this value is not equal to `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 1.xpect_not_eq(2);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is equal to `expected`.
	fn xpect_not_eq<U>(&self, expected: U) -> &Self
	where
		T: PartialEq<U>,
		U: Debug,
	{
		if self == &expected {
			assert_ext::panic_expected_received_display_debug(
				format!("NOT {:?}", expected),
				self,
			);
		}
		self
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_utils::utils::PipelineTarget;

	#[test]
	fn equality() {
		true.xpect_eq(true);
		true.xref().xpect_eq(true);

		"foo".xpect_eq("foo");
		"foo".to_string().xpect_eq("foo");

		"foo".xpect_not_eq("bar".to_string());
		"foo".to_string().xpect_not_eq("bar");
	}
}
