//! Cross-platform logging macros and their per-platform backends.
//!
//! Each backend routes by platform via [`cfg_if!`](crate::cfg_if): the browser
//! console on wasm, stdout/stderr on native std, and `tracing` on a bare no_std
//! target (no stdout, so the message still reaches the platform logger, eg RTT
//! on the esp32). The `cfg` checks live here in `beet_core`, where `std` is a
//! declared feature, so they are never evaluated in downstream crates.
use crate::prelude::*;

/// The per-platform backends behind [`cross_log!`](crate::cross_log),
/// [`cross_log_noline!`](crate::cross_log_noline) and
/// [`cross_log_error!`](crate::cross_log_error). Called by macro expansion, not
/// directly.
pub struct CrossLog;

impl CrossLog {
	/// Log a line, ignoring a stream that will not take it.
	///
	/// `println!` PANICS on a write error, which on a closed stdout (`| head`)
	/// would fault a program over the ordinary end of its own output. Raw
	/// output is best effort here for the same reason a log subscriber's is:
	/// nothing a caller of this could do about it. The one caller that must
	/// know is [`inline`](Self::inline), which is the streaming half.
	#[doc(hidden)]
	pub fn line(msg: &str) {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::log_1(&msg.into());
			} else if #[cfg(feature = "std")] {
				use std::io::Write;
				let _ = writeln!(std::io::stdout(), "{msg}");
			} else {
				tracing::info!("{msg}");
			}
		}
	}

	/// Log without a trailing newline, flushing after, and report whether the
	/// stream took it.
	///
	/// The one output call that answers, because it is the one whose caller
	/// can act: a body streamed chunk by chunk stops streaming when stdout is
	/// gone rather than reading the rest to write it nowhere.
	#[doc(hidden)]
	pub fn inline(msg: &str) -> Result {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::log_1(&msg.into());
				Ok(())
			} else if #[cfg(feature = "std")] {
				use std::io::Write;
				let mut stdout = std::io::stdout();
				write!(stdout, "{msg}")?;
				stdout.flush()?;
				Ok(())
			} else {
				tracing::info!("{msg}");
				Ok(())
			}
		}
	}

	/// Log a line to the error stream, ignoring a stream that will not take
	/// it: best effort for the same reason [`line`](Self::line) is.
	#[doc(hidden)]
	pub fn error(msg: &str) {
		crate::cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				crate::exports::web_sys::console::error_1(&msg.into());
			} else if #[cfg(feature = "std")] {
				use std::io::Write;
				let _ = writeln!(std::io::stderr(), "{msg}");
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
/// Answers with the write result: a closed stdout is the ordinary end of a
/// piped program's output, so a streaming caller stops rather than faulting.
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
