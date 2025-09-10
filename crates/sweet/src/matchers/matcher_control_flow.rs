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
