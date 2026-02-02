//! Result assertion matchers.
//!
//! This module provides assertion methods for [`Result`] types, including
//! checks for `Ok` and `Err` variants.

use crate::prelude::*;
use bevy::prelude::*;
use std::fmt::Debug;

/// Extension trait adding assertion methods to [`Result<T, E>`].
#[extend::ext(name=MatcherResult)]
pub impl<T: Debug, E: Debug> Result<T, E> {
	/// Performs an assertion ensuring this value is an `Ok(_)`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// Ok::<(), ()>(()).xpect_ok();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `Ok(_)`.
	#[track_caller]
	fn xpect_ok(&self) -> &Self {
		match self {
			Ok(_) => self,
			Err(_) => {
				panic_ext::panic_expected_received_display_debug("Ok", self);
			}
		}
	}
	/// Performs an assertion ensuring this value is an `Err(_)`.
	///
	/// ## Example
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// Err::<(), ()>(()).xpect_err();
	/// ```
	///
	/// ## Panics
	///
	/// Panics if the value is not `Err(_)`.
	#[track_caller]
	fn xpect_err(&self) -> &Self {
		match self {
			Err(_) => self,
			Ok(_) => {
				panic_ext::panic_expected_received_display_debug("Err", self);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;

	#[test]
	fn result() {
		let ok = || -> Result { Ok(()) };
		ok().xpect_ok();

		let err = || -> Result { Err("foo".into()) };

		err().xpect_err();
	}
}
