//! Poll a function until it finishes or a deadline expires.
//!
//! The wait primitive behind ui-style "wait until the element appears"
//! assertions, cross-platform (native, deno, browser). The cadence is a fixed
//! short interval rather than an exponential backoff: ui waits want steady
//! fast polls, [`Backoff`](crate::prelude::Backoff) remains the tool for
//! network-shaped retries.
//!
//! Two families, differing only in what the polled function says:
//!
//! - [`poll`] and its variants answer [`ControlFlow`] and are the default.
//!   `Break(value)` finishes, `Continue(())` waits out the interval and tries
//!   again, and `Err` fails the whole poll at once. Reach for these whenever
//!   an error can mean the thing being waited on is broken rather than
//!   merely late, so a bad selector or a refused handshake reports now
//!   instead of after the deadline.
//! - [`poll_result`] and its variants are the "any error means not ready
//!   yet" shorthand: every `Err` is a miss, retried until the deadline, and
//!   the last one becomes the timeout. For a probe whose only failure mode
//!   is earliness, ie an element that has not rendered, where an error type
//!   would carry no more information than its message already does.
//!
//! Each family takes a sync or an async probe. Only the sync ones can be
//! awaited from a future that must be `Send`, ie an `#[action]` body: see
//! [`poll_with`] for why an `async ||` probe cannot.

use crate::prelude::*;
use core::time::Duration;

/// The default poll deadline, chosen to sit under the 5s default test timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(4);
/// The default interval between poll attempts.
pub const DEFAULT_INTERVAL: Duration = Duration::from_millis(50);

/// Polls an async function until it breaks with a value, with the default
/// deadline and interval. See [`poll_async_with`].
pub async fn poll_async<T>(
	func: impl AsyncFnMut() -> Result<ControlFlow<T>>,
) -> Result<T> {
	poll_async_with(func, DEFAULT_TIMEOUT, DEFAULT_INTERVAL).await
}

/// Polls an async function until it breaks with a value, sleeping `interval`
/// between the attempts that answer `Continue` and erroring once `timeout`
/// expires. An `Err` is a genuine failure and returns immediately, without
/// waiting out the deadline.
///
/// The function is always attempted at least once, and a final attempt is
/// made at the deadline so a timeout never wins by a sleep's margin.
pub async fn poll_async_with<T>(
	mut func: impl AsyncFnMut() -> Result<ControlFlow<T>>,
	timeout: Duration,
	interval: Duration,
) -> Result<T> {
	let start = Instant::now();
	loop {
		let expired = start.elapsed() >= timeout;
		match func().await? {
			ControlFlow::Break(value) => return Ok(value),
			ControlFlow::Continue(()) if expired => {
				bevybail!("poll timed out after {timeout:?}")
			}
			ControlFlow::Continue(()) => time_ext::sleep(interval).await,
		}
	}
}

/// Polls a sync function until it breaks with a value, with the default
/// deadline and interval. See [`poll_async_with`].
pub async fn poll<T>(
	func: impl FnMut() -> Result<ControlFlow<T>>,
) -> Result<T> {
	poll_with(func, DEFAULT_TIMEOUT, DEFAULT_INTERVAL).await
}

/// Polls a sync function until it breaks with a value, bounded by `timeout`.
/// See [`poll_async_with`].
///
/// A loop of its own rather than [`poll_async_with`] over an `async ||`: that
/// closure's higher-ranked environment lifetime breaks `Send` inference for
/// callers whose futures must be `Send`, ie an `#[action]` body.
pub async fn poll_with<T>(
	mut func: impl FnMut() -> Result<ControlFlow<T>>,
	timeout: Duration,
	interval: Duration,
) -> Result<T> {
	let start = Instant::now();
	loop {
		let expired = start.elapsed() >= timeout;
		match func()? {
			ControlFlow::Break(value) => return Ok(value),
			ControlFlow::Continue(()) if expired => {
				bevybail!("poll timed out after {timeout:?}")
			}
			ControlFlow::Continue(()) => time_ext::sleep(interval).await,
		}
	}
}

/// Polls an async function until it returns `Ok`, with the default
/// deadline and interval, returning the last error on timeout.
pub async fn poll_result_async<T>(
	func: impl AsyncFnMut() -> Result<T>,
) -> Result<T> {
	poll_result_async_with(func, DEFAULT_TIMEOUT, DEFAULT_INTERVAL).await
}

/// Polls an async function until it returns `Ok` or `timeout` expires,
/// sleeping `interval` between attempts, returning the last error on timeout.
/// The function is always attempted at least once, and a final attempt is
/// made at the deadline so a timeout never wins by a sleep's margin.
pub async fn poll_result_async_with<T>(
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
pub async fn poll_result<T>(func: impl FnMut() -> Result<T>) -> Result<T> {
	poll_result_with(func, DEFAULT_TIMEOUT, DEFAULT_INTERVAL).await
}

/// Polls a sync function until it returns `Ok` or `timeout` expires,
/// sleeping `interval` between attempts, returning the last error on timeout.
/// A loop of its own for the same `Send` reason as [`poll_with`].
pub async fn poll_result_with<T>(
	mut func: impl FnMut() -> Result<T>,
	timeout: Duration,
	interval: Duration,
) -> Result<T> {
	let start = Instant::now();
	loop {
		let expired = start.elapsed() >= timeout;
		match func() {
			Ok(value) => return Ok(value),
			Err(err) if expired => return Err(err),
			Err(_) => time_ext::sleep(interval).await,
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[crate::test]
	async fn breaks_with_a_value_immediately() {
		let mut attempts = 0;
		poll(|| {
			attempts += 1;
			ControlFlow::Break(attempts).xok()
		})
		.await
		.unwrap()
		.xpect_eq(1);
	}

	#[crate::test]
	async fn continues_until_the_condition_holds() {
		let mut attempts = 0;
		poll(|| {
			attempts += 1;
			match attempts >= 3 {
				true => ControlFlow::Break(attempts),
				false => ControlFlow::Continue(()),
			}
			.xok()
		})
		.await
		.unwrap()
		.xpect_eq(3);
	}

	#[crate::test]
	async fn an_error_returns_without_waiting_out_the_deadline() {
		let start = Instant::now();
		poll_with(
			|| Err::<ControlFlow<()>, BevyError>(bevyhow!("broken")),
			Duration::from_secs(10),
			Duration::from_millis(10),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("broken");
		start.elapsed().xpect_less_than(Duration::from_secs(1));
	}

	#[crate::test]
	async fn errors_when_the_deadline_expires() {
		poll_with(
			|| Ok::<ControlFlow<()>, BevyError>(ControlFlow::Continue(())),
			Duration::from_millis(30),
			Duration::from_millis(10),
		)
		.await
		.unwrap_err()
		.to_string()
		.xpect_contains("timed out");
	}

	#[crate::test]
	async fn poll_result_succeeds_once_the_condition_holds() {
		let mut attempts = 0;
		poll_result(|| {
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
	async fn poll_result_returns_the_last_error_on_timeout() {
		poll_result_with(
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
