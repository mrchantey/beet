use extend::ext;

#[cfg(target_arch = "wasm32")]
use crate::web_utils::js_runtime;

/// Extension trait for [`Result`] providing additional conversion methods.
#[ext]
pub impl<T, E> Result<T, E> {
	/// Converts to `Option<T>`, calling `func` with the error if `Err`.
	///
	/// This is useful when you want to handle an error for side effects
	/// (like logging) while still converting to `Option`.
	///
	/// # Examples
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let ok_result: Result<i32, &str> = Ok(42);
	/// let value = ok_result.ok_or(|e| eprintln!("Error: {e}"));
	/// assert_eq!(value, Some(42));
	///
	/// let err_result: Result<i32, &str> = Err("failed");
	/// let mut logged = false;
	/// let value = err_result.ok_or(|_| logged = true);
	/// assert_eq!(value, None);
	/// assert!(logged);
	/// ```
	fn ok_or(self, func: impl FnOnce(E)) -> Option<T> {
		match self {
			Ok(value) => Some(value),
			Err(err) => {
				func(err);
				None
			}
		}
	}
}



/// Extension trait for [`Result`] with displayable errors.
#[ext(name=ResultExtDisplay)]
pub impl<T, E: std::fmt::Display> Result<T, E> {
	/// Unwraps the value or exits the process with a formatted error message.
	///
	/// Unlike [`unwrap`](Result::unwrap), this prints a user-friendly error
	/// message instead of a panic. On native platforms, exits with code 1.
	/// On wasm, calls the JavaScript runtime exit function.
	///
	/// # Examples
	///
	/// ```no_run
	/// # use beet_core::prelude::*;
	/// let result: Result<i32, &str> = Ok(42);
	/// let value = result.unwrap_or_exit(); // Returns 42
	///
	/// let result: Result<i32, &str> = Err("something went wrong");
	/// let value = result.unwrap_or_exit(); // Prints error and exits
	/// ```
	fn unwrap_or_exit(self) -> T {
		match self {
			Ok(value) => value,
			Err(err) => {
				eprintln!("{err}");
				#[cfg(not(target_arch = "wasm32"))]
				std::process::exit(1);
				#[cfg(target_arch = "wasm32")]
				{
					js_runtime::exit(1);
					#[allow(clippy::empty_loop)]
					loop {}
				}
			}
		}
	}
}
