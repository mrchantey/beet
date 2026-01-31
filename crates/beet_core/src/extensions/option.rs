use crate::prelude::*;
use extend::ext;

/// Extension trait for [`Option`] providing additional conversion methods.
#[ext]
pub impl<T> Option<T> {
	/// Converts `Some(value)` to `Ok(value)`, or returns a generic error for `None`.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let some_value: Option<i32> = Some(42);
	/// assert!(some_value.or_err().is_ok());
	///
	/// let none_value: Option<i32> = None;
	/// assert!(none_value.or_err().is_err());
	/// ```
	fn or_err(self) -> Result<T> {
		match self {
			Some(value) => Ok(value),
			None => Err(bevyhow!("Expected Some")),
		}
	}
}
