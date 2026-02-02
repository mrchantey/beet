//! Polling utilities for async operations with timeouts.
//!
//! Provides helpers for repeatedly polling a fallible function until it
//! succeeds or a timeout expires.

use super::performance_now;
use crate::prelude::*;
use std::time::Duration;

/// Polls a function until it returns `Ok`, with a default 2-second timeout.
///
/// Calls `f` repeatedly with 10ms delays until success or timeout.
///
/// # Errors
///
/// Returns the last error if the timeout expires before success.
pub async fn poll_ok<T>(f: impl FnMut() -> Result<T>) -> Result<T> {
	poll_ok_with_timeout(f, Duration::from_secs(2)).await
}

/// Polls a function until it returns `Ok` or the timeout expires.
///
/// Calls `f` repeatedly with 10ms delays between attempts.
///
/// # Errors
///
/// Returns the last error if the timeout expires before success.
pub async fn poll_ok_with_timeout<T>(
	mut f: impl FnMut() -> Result<T>,
	timeout: Duration,
) -> Result<T> {
	let start = performance_now();
	loop {
		match f() {
			Ok(val) => return Ok(val),
			Err(err) => {
				if performance_now() - start > timeout.as_millis() as f64 {
					return Err(err);
				}
				time_ext::sleep(Duration::from_millis(10)).await;
			}
		}
	}
}
