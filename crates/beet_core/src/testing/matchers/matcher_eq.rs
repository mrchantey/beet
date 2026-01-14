use crate::prelude::*;
use std::fmt::Debug;

#[extend::ext(name=MatcherEq)]
pub impl<T> T
where
	T: Debug,
{
	/// Performs an assertion ensuring this value is equal to `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// 1.xpect_eq(1);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not equal to `expected`.
	#[track_caller]
	fn xpect_eq<U>(&self, expected: U) -> &Self
	where
		T: PartialEq<U>,
		U: Debug,
	{
		if self != &expected {
			panic_ext::panic_expected_received_debug(expected, self);
		}
		self
	}
	/// Performs an assertion ensuring this value is not equal to `expected`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// 1.xpect_not_eq(2);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is equal to `expected`.
	#[track_caller]
	fn xpect_not_eq<U>(&self, expected: U) -> &Self
	where
		T: PartialEq<U>,
		U: Debug,
	{
		if self == &expected {
			panic_ext::panic_expected_received_display_debug(
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

	#[test]
	fn equality() {
		true.xpect_eq(true);
		(&true).xpect_eq(true);

		"foo".xpect_eq("foo");
		"foo".to_string().xpect_eq("foo");

		"foo".xpect_not_eq("bar".to_string());
		"foo".to_string().xpect_not_eq("bar");
	}
}
