//! Control flow assertion matchers.
//!
//! This module provides assertion methods for [`ControlFlow`] values.

use crate::prelude::*;
use std::fmt::Debug;
use std::ops::ControlFlow;

/// Extension trait adding assertion methods to [`ControlFlow<B, C>`].
#[extend::ext(name=MatcherControlFlow)]
pub impl<B: Debug, C: Debug> ControlFlow<B, C> {
	/// Performs an assertion ensuring this value is a `ControlFlow::Continue`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// # use std::ops::ControlFlow;
	/// ControlFlow::<(),()>::Continue(()).xpect_continue();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `ControlFlow::Continue`.
	#[track_caller]
	fn xpect_continue(&self) -> &Self {
		match self {
			ControlFlow::Continue(_) => self,
			ControlFlow::Break(_) => {
				panic_ext::panic_expected_received_display_debug(
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
	/// # use beet_core::prelude::*;
	/// # use std::ops::ControlFlow;
	/// ControlFlow::<(),()>::Break(()).xpect_break();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `ControlFlow::Break`.
	#[track_caller]
	fn xpect_break(&self) -> &Self {
		match self {
			ControlFlow::Break(_) => self,
			ControlFlow::Continue(_) => {
				panic_ext::panic_expected_received_display_debug("Break", self);
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
