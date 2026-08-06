//! Cross-platform logging macros and their per-platform backends.
//!
//! Each backend routes by platform via [`cfg_if!`](crate::cfg_if): the browser
//! console on wasm, stdout/stderr on native std, and `tracing` on a bare no_std
//! target (no stdout, so the message still reaches the platform logger, eg RTT
//! on the esp32). The `cfg` checks live here in `beet_core`, where `std` is a
//! declared feature, so they are never evaluated in downstream crates.

/// The per-platform backends behind [`cross_log!`](crate::cross_log),
/// [`cross_log_noline!`](crate::cross_log_noline) and
/// [`cross_log_error!`](crate::cross_log_error). Called by macro expansion, not
/// directly.
pub struct CrossLog;

impl CrossLog {
	/// Log a line.
	#[doc(hidden)]
	pub fn line(msg: &str) {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::log_1(&msg.into());
			} else if #[cfg(feature = "std")] {
				println!("{msg}");
			} else {
				tracing::info!("{msg}");
			}
		}
	}

	/// Log without a trailing newline, flushing after.
	#[doc(hidden)]
	pub fn inline(msg: &str) {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::log_1(&msg.into());
			} else if #[cfg(feature = "std")] {
				use std::io::Write;
				print!("{msg}");
				std::io::stdout().flush().unwrap();
			} else {
				tracing::info!("{msg}");
			}
		}
	}

	/// Log a line to the error stream.
	#[doc(hidden)]
	pub fn error(msg: &str) {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::error_1(&msg.into());
			} else if #[cfg(feature = "std")] {
				eprintln!("{msg}");
			} else {
				tracing::error!("{msg}");
			}
		}
	}
}

/// Cross-platform raw output without a trailing newline.
///
/// Only for output that must not carry a log prefix, ie streaming a response
/// body to stdout or rendering the program's actual result. Never for
/// informational logging, which uses the `log` crate (`error!`/`warn!`/`info!`/
/// `debug!`), already cross-platform via the `log` facade + the app's `LogPlugin`.
///
/// - **wasm32**: writes to `console.log`
/// - **native + std**: prints to stdout and flushes
/// - **native + no_std**: records via `tracing`
#[macro_export]
macro_rules! cross_log_noline {
	($($t:tt)*) => {
		$crate::CrossLog::inline(&$crate::_alloc::format!($($t)*))
	};
}

/// Cross-platform raw output with a trailing newline.
///
/// Only for output that must not carry a log prefix, ie streaming a response
/// body to stdout or rendering the program's actual result. Never for
/// informational logging, which uses the `log` crate (`error!`/`warn!`/`info!`/
/// `debug!`), already cross-platform via the `log` facade + the app's `LogPlugin`.
///
/// - **wasm32**: writes to `console.log`
/// - **native + std**: prints to stdout
/// - **native + no_std**: records via `tracing`
#[macro_export]
macro_rules! cross_log {
	($($t:tt)*) => {
		$crate::CrossLog::line(&$crate::_alloc::format!($($t)*))
	};
}

/// Cross-platform error logging with a trailing newline.
///
/// - **wasm32**: writes to `console.error`
/// - **native + std**: prints to stderr
/// - **native + no_std**: records via `tracing`
#[macro_export]
macro_rules! cross_log_error {
	($($t:tt)*) => {
		$crate::CrossLog::error(&$crate::_alloc::format!($($t)*))
	};
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
