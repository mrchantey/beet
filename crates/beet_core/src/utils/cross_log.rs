//! Cross-platform logging macros and their native backend helpers.

/// Internal helper: log a line to native stdout.
///
/// The `cfg(feature = "std")` check lives here in `beet_core`, where `std`
/// is a declared feature, so it is never evaluated in downstream crates.
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub fn _cross_log_native(msg: &str) {
	#[cfg(feature = "std")]
	println!("{}", msg);
}

/// Internal helper: log without a newline to native stdout, flushing after.
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub fn _cross_log_native_noline(msg: &str) {
	#[cfg(feature = "std")]
	{
		print!("{}", msg);
		use std::io::Write;
		std::io::stdout().flush().unwrap();
	}
}

/// Internal helper: log a line to native stderr.
#[cfg(not(target_arch = "wasm32"))]
#[doc(hidden)]
pub fn _cross_log_error_native(msg: &str) {
	#[cfg(feature = "std")]
	eprintln!("{}", msg);
}

/// Cross-platform logging without a trailing newline.
///
/// - **wasm32**: writes to `console.log`
/// - **native + std**: prints to stdout and flushes
/// - **native + no_std**: no-op
#[macro_export]
macro_rules! cross_log_noline {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
      $crate::exports::web_sys::console::log_1(
          &($crate::_alloc::format!($($t)*).into())
      );
      #[cfg(not(target_arch = "wasm32"))]
      $crate::_cross_log_native_noline(&$crate::_alloc::format!($($t)*));
    })
}

/// Cross-platform logging with a trailing newline.
///
/// - **wasm32**: writes to `console.log`
/// - **native + std**: prints to stdout
/// - **native + no_std**: no-op
#[macro_export]
macro_rules! cross_log {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
      $crate::exports::web_sys::console::log_1(
          &($crate::_alloc::format!($($t)*).into())
      );
      #[cfg(not(target_arch = "wasm32"))]
      $crate::_cross_log_native(&$crate::_alloc::format!($($t)*));
    })
}

/// Cross-platform error logging with a trailing newline.
///
/// - **wasm32**: writes to `console.error`
/// - **native + std**: prints to stderr
/// - **native + no_std**: no-op
#[macro_export]
macro_rules! cross_log_error {
    ($($t:tt)*) => ({
      #[cfg(target_arch = "wasm32")]
      $crate::exports::web_sys::console::error_1(
          &($crate::_alloc::format!($($t)*).into())
      );
      #[cfg(not(target_arch = "wasm32"))]
      $crate::_cross_log_error_native(&$crate::_alloc::format!($($t)*));
    })
}

/// Logs the current source location, ie `file!():line!():column!()`.
#[macro_export]
macro_rules! breakpoint {
	() => {{
		$crate::cross_log!(
			"breakpoint at {}:{}:{}",
			file!(),
			line!(),
			column!()
		);
	}};
}
