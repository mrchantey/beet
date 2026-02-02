//! Boolean assertion matchers.
//!
//! This module provides assertion methods for boolean values.

use crate::prelude::*;

/// Extension trait adding assertion methods to [`bool`].
#[extend::ext(name=MatcherBool)]
pub impl bool {
	/// Performs an assertion ensuring this value is equal to `true`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// true.xpect_true();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `true`.
	#[track_caller]
	fn xpect_true(&self) -> &Self {
		panic_ext::assert_expected_received_display(&true, self);
		self
	}
	/// Performs an assertion ensuring this value is equal to `false`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// false.xpect_false();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `false`.
	#[track_caller]
	fn xpect_false(&self) -> &Self {
		panic_ext::assert_expected_received_display(&false, self);
		self
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn bool() {
		// false.xpect_true();
		true.xpect_true();
		false.xpect_false();
	}
}
