use super::*;
use std::fmt::Debug;
use std::ops::ControlFlow;

#[extend::ext(name=SweetControlFlow)]
pub impl<B: Debug, C: Debug> ControlFlow<B, C> {
	/// Performs an assertion ensuring this value is a `ControlFlow::Continue`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// # use std::ops::ControlFlow;
	/// ControlFlow::<(),()>::Continue(()).xpect_continue();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `ControlFlow::Continue`.
	fn xpect_continue(&self) -> &Self {
		match self {
			ControlFlow::Continue(_) => self,
			ControlFlow::Break(_) => {
				assert_ext::panic_expected_received_display_debug(
					"Continue", self,
				);
			}
		}
	}

	/// Performs an assertion ensuring this value is a `ControlFlow::Break`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// # use std::ops::ControlFlow;
	/// ControlFlow::<(),()>::Break(()).xpect_break();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `ControlFlow::Break`.
	fn xpect_break(&self) -> &Self {
		match self {
			ControlFlow::Break(_) => self,
			ControlFlow::Continue(_) => {
				assert_ext::panic_expected_received_display_debug(
					"Break", self,
				);
			}
		}
	}
}



impl<B, C> Matcher<ControlFlow<B, C>> {
	pub fn to_continue(&self) {
		match &self.value {
			ControlFlow::Continue(_) => {}
			ControlFlow::Break(_) => {
				self.assert_correct_with_received(
					false,
					&"ControlFlow::Continue",
					&"ControlFlow::Break",
				);
			}
		}
	}
	pub fn to_break(&self) {
		match &self.value {
			ControlFlow::Break(_) => {}
			ControlFlow::Continue(_) => {
				self.assert_correct_with_received(
					false,
					&"ControlFlow::Break",
					&"ControlFlow::Continue",
				);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use std::ops::ControlFlow;

	use crate::prelude::*;

	#[test]
	fn works() {
		ControlFlow::<(), ()>::Continue(()).xpect_continue();
		ControlFlow::<(), ()>::Break(()).xpect_break();
	}
}
