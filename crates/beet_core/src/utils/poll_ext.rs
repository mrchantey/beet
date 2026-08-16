//! Poll a fallible function until it succeeds or a deadline expires.
//!
//! The wait primitive behind ui-style "wait until the element appears"
//! assertions, cross-platform (native, deno, browser). The cadence is a fixed
//! short interval rather than an exponential backoff: ui waits want steady
//! fast polls, [`Backoff`](crate::prelude::Backoff) remains the tool for
//! network-shaped retries.

use crate::prelude::*;
use core::time::Duration;

/// The default poll deadline, chosen to sit under the 5s default test timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(4);
/// The default interval between poll attempts.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(50);

/// Polls an async function until it returns `Ok`, with the default
/// deadline and interval, returning the last error on timeout.
pub async fn poll_async<T>(func: impl AsyncFnMut() -> Result<T>) -> Result<T> {
	poll_async_with(func, DEFAULT_TIMEOUT, DEFAULT_INTERVAL).await
}

/// Polls an async function until it returns `Ok` or `timeout` expires,
/// sleeping `interval` between attempts, returning the last error on timeout.
/// The function is always attempted at least once, and a final attempt is
/// made at the deadline so a timeout never wins by a sleep's margin.
pub async fn poll_async_with<T>(
	mut func: impl AsyncFnMut() -> Result<T>,
	timeout: Duration,
	interval: Duration,
) -> Result<T> {
	let start = Instant::now();
	loop {
		let expired = start.elapsed() >= timeout;
		match func().await {
			Ok(value) => return Ok(value),
			Err(err) if expired => return Err(err),
			Err(_) => time_ext::sleep(interval).await,
		}
	}
}

/// Polls a sync function until it returns `Ok`, with the default deadline
/// and interval, returning the last error on timeout.
pub async fn poll<T>(mut func: impl FnMut() -> Result<T>) -> Result<T> {
	poll_async(async || func()).await
}

/// Polls a sync function until it returns `Ok` or `timeout` expires,
/// sleeping `interval` between attempts, returning the last error on timeout.
pub async fn poll_with<T>(
	mut func: impl FnMut() -> Result<T>,
	timeout: Duration,
	interval: Duration,
) -> Result<T> {
	poll_async_with(async || func(), timeout, interval).await
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	async fn succeeds_once_the_condition_holds() {
		let mut attempts = 0;
		poll(|| {
			attempts += 1;
			if attempts >= 3 {
				Ok(attempts)
			} else {
				Err(bevyhow!("not yet"))
			}
		})
		.await
		.unwrap()
		.xpect_eq(3);
	}

	#[crate::test]
	async fn returns_the_last_error_on_timeout() {
		poll_with(
			|| Err::<(), _>(bevyhow!("never")),
			Duration::from_millis(30),
			Duration::from_millis(10),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("never");
	}
}
