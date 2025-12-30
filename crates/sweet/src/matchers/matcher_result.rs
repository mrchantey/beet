use super::*;
use std::fmt::Debug;

#[extend::ext(name=SweetResult)]
pub impl<T: Debug, E: Debug> Result<T, E> {
	/// Performs an assertion ensuring this value is an `Ok(_)`.
	///
	/// ## Example
	///
	/// ```
	/// # use sweet::prelude::*;
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
	/// # use sweet::prelude::*;
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
	use anyhow::anyhow;

	#[test]
	fn result() {
		let ok = || -> anyhow::Result<()> { Ok(()) };
		ok().xpect_ok();

		let err = || -> anyhow::Result<()> { Err(anyhow!("foo")) };

		err().xpect_err();
	}
}
