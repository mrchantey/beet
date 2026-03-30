//! Bevy-compatible error macros similar to `anyhow`.
//!
//! This module provides [`bevyhow!`](crate::bevyhow) and [`bevybail!`](crate::bevybail)
//! macros for creating errors compatible with Bevy's [`BevyError`] type.

use crate::prelude::*;
use alloc::string::String;
use bevy::prelude::BevyError;

/// Trait for converting a value or Result into a `Result`.
pub trait IntoResult<T = (), E = BevyError> {
	/// Converts this value into a `Result`.
	fn into_result(self) -> Result<T, E>;
}
impl<T, E> IntoResult<T, E> for T {
	fn into_result(self) -> Result<T, E> { self.xok() }
}
impl<T, E> IntoResult<T, E> for Result<T, E> {
	fn into_result(self) -> Result<T, E> { self }
}


/// Intermediary type for converting formatted strings to [`BevyError`].
///
/// This type implements [`core::error::Error`] and can be converted into a
/// [`BevyError`] for use in Bevy systems.
pub struct BevyhowError {
	/// The error message
	pub message: String,
	/// The location where the error was created, captured using `#[track_caller]`.
	pub location: &'static core::panic::Location<'static>,
}

impl BevyhowError {
	/// Creates a new error from any string-like type.
	#[track_caller]
	pub fn new(msg: impl Into<String>) -> Self {
		BevyhowError {
			message: msg.into(),
			location: core::panic::Location::caller(),
		}
	}

	/// Converts this error into a [`BevyError`].
	pub fn into_bevy(self) -> BevyError { self.into() }
}

impl core::error::Error for BevyhowError {}

impl core::fmt::Display for BevyhowError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		let location = format!("  at {}", self.location);
		#[cfg(feature = "ansi_paint")]
		let location = paint_ext::dimmed(location);
		write!(f, "{}\n{}", self.message, location)
	}
}

impl core::fmt::Debug for BevyhowError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "{}\n\tat {}", self.message, self.location)
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
			$crate::prelude::BevyhowError::new($crate::_alloc::format!($($arg)*)).into_bevy()
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
		return Err($crate::prelude::BevyhowError::new($crate::_alloc::format!($($arg)*)).into_bevy())
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
