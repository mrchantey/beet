use super::*;
use std::fmt::Debug;
#[extend::ext(name=SweetVec)]
pub impl<T: Debug> Vec<T> {
	/// Performs an assertion ensuring at least one item in the vec passes the predicate.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// vec![false, true].xpect_any(|v| *v == true);
	/// ```
	///
	/// ## Panics
	///
	/// Panics if no items pass.
	#[track_caller]
	fn xpect_any(&self, func: impl Fn(&T) -> bool) -> &Self {
		if !self.iter().any(|item| func(item)) {
			panic_ext::panic_expected_received_display_debug(
				"Any item to pass predicate",
				self,
			);
		}

		self
	}
	/// Performs an assertion ensuring the `Vec` is empty.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// Vec::<u32>::new().xpect_empty();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the `Vec` is not empty.
	#[track_caller]
	fn xpect_empty(&self) -> &Self {
		if !self.is_empty() {
			panic_ext::panic_expected_received_display_debug(
				"To be empty",
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
	#[should_panic]
	fn xpect_any_panics() { vec![false, false].xpect_any(|v| *v == true); }
	#[test]
	fn xpect_any() { vec![false, true].xpect_any(|v| *v == true); }
	#[test]
	fn xpect_empty() { Vec::<()>::default().xpect_empty(); }
}
