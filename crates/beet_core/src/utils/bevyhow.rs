//! Bevy-compatible error macros similar to `anyhow`.
//!
//! This module provides [`bevyhow!`](crate::bevyhow) and [`bevybail!`](crate::bevybail)
//! macros for creating errors compatible with Bevy's [`BevyError`] type.

use bevy::prelude::BevyError;

/// Intermediary type for converting formatted strings to [`BevyError`].
///
/// This type implements [`std::error::Error`] and can be converted into a
/// [`BevyError`] for use in Bevy systems.
pub struct BevyhowError(pub String);

impl BevyhowError {
	/// Creates a new error from any string-like type.
	pub fn new(msg: impl Into<String>) -> Self { BevyhowError(msg.into()) }

	/// Converts this error into a [`BevyError`].
	pub fn into_bevy(self) -> BevyError { self.into() }
}

impl std::error::Error for BevyhowError {}

impl std::fmt::Display for BevyhowError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

impl std::fmt::Debug for BevyhowError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", self.0)
	}
}

/// Creates a [`BevyError`](bevy::ecs::error::BevyError) with formatted arguments.
///
/// This is similar to [`anyhow::anyhow!`] but produces a Bevy-compatible error type.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// # use bevy::ecs::error::BevyError;
/// let err: BevyError = bevyhow!("something went wrong");
/// let err: BevyError = bevyhow!("failed with code {}", 42);
/// ```
#[macro_export]
macro_rules! bevyhow {
		($($arg:tt)*) => {
			$crate::prelude::BevyhowError::new(std::format!($($arg)*)).into_bevy()
		};
}

/// Early returns with a [`BevyError`](bevy::ecs::error::BevyError).
///
/// This is similar to [`anyhow::bail!`] but produces a Bevy-compatible error type.
///
/// # Examples
///
/// ```
/// # use beet_core::prelude::*;
/// # use bevy::prelude::*;
/// fn validate(value: i32) -> Result {
///     if value < 0 {
///         bevybail!("value must be non-negative, got {}", value);
///     }
///     Ok(())
/// }
/// ```
#[macro_export]
macro_rules! bevybail {
	($($arg:tt)*) => {
		return Err($crate::prelude::BevyhowError::new(std::format!($($arg)*)).into_bevy())
	};
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::ecs::error::BevyError;


	#[test]
	fn works() {
		let foo = 1;
		let bar = 2;
		let a: BevyError = bevyhow!("literal");
		let b: BevyError = bevyhow!("fmt literal inline {foo}{bar}");
		let c: BevyError = bevyhow!("fmt literal {}{}", 1, 2);
		a.to_string().xpect_starts_with("literal\n");
		b.to_string().xpect_starts_with("fmt literal inline 12\n");
		c.to_string().xpect_starts_with("fmt literal 12\n");

		let a = || -> Result {
			bevybail!("literal");
		};
		let b = || -> Result {
			bevybail!("fmt literal inline {foo}{bar}");
		};
		let c = || -> Result {
			bevybail!("fmt literal {}{}", 1, 2);
		};

		a().unwrap_err().to_string().xpect_starts_with("literal\n");
		b().unwrap_err()
			.to_string()
			.xpect_starts_with("fmt literal inline 12\n");
		c().unwrap_err()
			.to_string()
			.xpect_starts_with("fmt literal 12\n");
	}
}
