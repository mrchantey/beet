use super::*;
use std::fmt::Debug;

#[extend::ext(name=SweetOrd)]
pub impl<T: PartialOrd + Debug + Copy> T {
	/// Performs an assertion ensuring this value is less than `other`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 1.xpect_less_than(2);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not less than `other`.
	#[track_caller]
	fn xpect_less_than(&self, other: T) -> &Self {
		if *self < other {
			self
		} else {
			panic_ext::panic_expected_received_display_debug(
				&format!("less than {:?}", other),
				self,
			);
		}
	}
	/// Performs an assertion ensuring this value is less or equal to `other`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 2.xpect_less_or_equal_to(2);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not less or equal to `other`.
	#[track_caller]
	fn xpect_less_or_equal_to(&self, other: T) -> &Self {
		if *self <= other {
			self
		} else {
			panic_ext::panic_expected_received_display_debug(
				&format!("less or equal to {:?}", other),
				self,
			);
		}
	}
	/// Performs an assertion ensuring this value is greater than `other`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 3.xpect_greater_than(2);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not greater than `other`.
	#[track_caller]
	fn xpect_greater_than(&self, other: T) -> &Self {
		if *self > other {
			self
		} else {
			panic_ext::panic_expected_received_display_debug(
				&format!("greater than {:?}", other),
				self,
			);
		}
	}
	/// Performs an assertion ensuring this value is greater or equal to `other`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// 3.xpect_greater_or_equal_to(3);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not greater or equal to `other`.
	#[track_caller]
	fn xpect_greater_or_equal_to(&self, other: T) -> &Self {
		if *self >= other {
			self
		} else {
			panic_ext::panic_expected_received_display_debug(
				&format!("greater or equal to {:?}", other),
				self,
			);
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;


	#[test]
	fn order() {
		0.xpect_greater_or_equal_to(0);
		10.xpect_greater_than(-10);
		10.xpect_less_or_equal_to(11);
	}
}
