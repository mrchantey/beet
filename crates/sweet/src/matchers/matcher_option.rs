use super::*;
use std::fmt::Debug;

#[extend::ext(name=SweetOption)]
pub impl<T: Debug> Option<T> {
	/// Performs an assertion ensuring this value is a `Some(_)`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// Some(1).xpect_some();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `Some(_)`.
	fn xpect_some(&self) -> &Self {
		match self {
			Some(_) => self,
			None => {
				assert_ext::panic_expected_received_display_debug("Some", self);
			}
		}
	}
	/// Performs an assertion ensuring this value is a `None`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
	/// let v: Option<i32> = None;
	/// v.xpect_none();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `None`.
	fn xpect_none(&self) -> &Self {
		match self {
			None => self,
			Some(_) => {
				assert_ext::panic_expected_received_display_debug("None", self);
			}
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[test]
	fn option() {
		Some(true).xpect_some();
		(None::<bool>).xpect_none();
	}
}
