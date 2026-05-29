//! Time utilities for cross-platform duration handling and async sleep.
//!
//! [`pretty_print_duration`] is pure formatting and works on no_std; the clock
//! and sleep helpers are std-only (per-function gated, not whole-module).

use crate::prelude::*;
use bevy::platform::sync::OnceLock;
use core::time::Duration;
#[cfg(feature = "std")]
use std::time::SystemTime;

/// A wall-clock source: the current time as a [`Duration`] since the Unix epoch.
///
/// This is the no_std-friendly clock hook, mirroring `Instant::set_elapsed` (for
/// the monotonic clock) and `set_http_client` (for transport). On a bare target
/// there is no `SystemTime`, so [`now`]/[`try_now`] fall back to a function
/// installed via [`set_now`] — letting a downstream adapter (eg an SNTP client)
/// supply wall-clock time once it has it.
pub type NowFn = fn() -> Duration;

static NOW: OnceLock<NowFn> = OnceLock::new();

/// Install the wall-clock source used by [`now`] and [`try_now`].
///
/// Call once the source is ready (eg after an SNTP sync). Takes precedence over
/// the std `SystemTime` fallback, so it can also be used to mock time. Returns
/// an error if a source has already been installed.
pub fn set_now(now: NowFn) -> Result<()> {
	NOW.set(now)
		.map_err(|_| bevyhow!("a `now` clock source is already installed"))
}

/// The current time as a [`Duration`] since the Unix epoch, or an error if no
/// clock is available yet.
///
/// Resolution order:
/// 1. a source installed via [`set_now`];
/// 2. on `std`, the system clock (`SystemTime`);
/// 3. otherwise an error — eg a bare target whose SNTP client hasn't synced.
///
/// Prefer this over [`now`] when the clock may still be loading.
pub fn try_now() -> Result<Duration> {
	if let Some(now) = NOW.get() {
		return Ok(now());
	}
	cfg_if! {
		if #[cfg(feature = "std")] {
			SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map_err(|err| bevyhow!("system clock is before the Unix epoch: {err}"))
		} else {
			bevybail!(
				"no wall clock available yet; install one with `set_now` \
				 (eg once SNTP has synced)"
			)
		}
	}
}

/// The current time as a [`Duration`] since the Unix epoch.
///
/// # Panics
///
/// Panics if no clock is available (see [`try_now`] for the fallible form).
pub fn now() -> Duration {
	try_now().expect(
		"no wall clock available; install one with `set_now` or use `try_now`",
	)
}

/// Formats a duration as a human-readable string with appropriate units.
///
/// Automatically selects the most appropriate unit (minutes, seconds,
/// milliseconds, microseconds, or nanoseconds) based on the duration's magnitude.
pub fn pretty_print_duration(dur: Duration) -> String {
	let total_secs = dur.as_secs();
	let minutes = total_secs / 60;
	let secs = total_secs % 60;
	let millis = dur.subsec_millis();
	if minutes > 0 {
		format!("{}:{:02}.{:03} m", minutes, secs, millis)
	} else if secs > 0 {
		format!("{}.{:02} s", secs, millis)
	} else if millis > 0 {
		format!("{} ms", millis)
	} else {
		let micros = dur.subsec_micros();
		if micros > 0 {
			format!("{} µs", micros)
		} else {
			let nanos = dur.subsec_nanos();
			format!("{} ns", nanos)
		}
	}
}

/// Returns the current time as milliseconds since the Unix epoch.
#[cfg(feature = "std")]
pub fn now_millis() -> u128 {
	SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap()
		.as_millis()
}

/// Sleeps for the specified number of seconds.
#[cfg(feature = "std")]
pub async fn sleep_secs(secs: u64) { sleep(Duration::from_secs(secs)).await; }

/// Sleeps for the specified number of milliseconds.
#[cfg(feature = "std")]
pub async fn sleep_millis(millis: u64) {
	sleep(Duration::from_millis(millis)).await;
}

/// Sleeps for the specified number of microseconds.
#[cfg(feature = "std")]
pub async fn sleep_micros(micros: u64) {
	sleep(Duration::from_micros(micros)).await;
}

/// Cross platform sleep function
#[cfg(feature = "std")]
#[allow(unused)]
pub async fn sleep(duration: Duration) {
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			use wasm_bindgen_futures::JsFuture;
			use web_sys::window;
			let window = window().unwrap();
			let promise = js_sys::Promise::new(&mut |resolve, _| {
				window
					.set_timeout_with_callback_and_timeout_and_arguments_0(
						&resolve,
						duration.as_millis() as i32,
					)
					.expect("should register `setTimeout` OK");
			});

			JsFuture::from(promise)
				.await
				.expect("should await `setTimeout` OK");
		} else {
			async_io::Timer::after(duration).await;
		}
	}
}



/// Runs a Send+Sync function with a timeout on native platforms.
/// Returns `Ok(PanicResult)` if completed, `Err(elapsed)` if timed out.
///
/// On native, spawns the function in a thread and uses `recv_timeout`.
/// On WASM, cannot enforce hard timeouts for sync code, so this is not available.
#[cfg(all(feature = "std", not(target_arch = "wasm32")))]
pub fn timeout_sync(
	func: impl 'static + Send + Sync + FnOnce() -> Result<(), String>,
	timeout: Duration,
) -> Result<PanicResult, Duration> {
	use std::sync::mpsc;

	let (sender, receiver) = mpsc::channel();
	let timeout_start = Instant::now();

	std::thread::spawn(move || {
		let _ = sender.send(PanicContext::catch(func));
	});

	match receiver.recv_timeout(timeout) {
		Ok(result) => Ok(result),
		Err(mpsc::RecvTimeoutError::Timeout) => Err(timeout_start.elapsed()),
		Err(mpsc::RecvTimeoutError::Disconnected) => {
			Ok(PanicResult::Err("Thread disconnected unexpectedly".into()))
		}
	}
}


// every test here exercises std-only sleep/timeout helpers
#[cfg(all(test, feature = "std"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	async fn works() {
		let now = Instant::now();
		time_ext::sleep(Duration::from_millis(100)).await;
		now.elapsed().as_millis().xpect_greater_or_equal_to(100);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn timeout_sync_completes() {
		time_ext::timeout_sync(|| Ok(()), Duration::from_millis(100))
			.unwrap()
			.xpect_eq(PanicResult::Ok);
	}

	#[cfg(not(target_arch = "wasm32"))]
	#[crate::test]
	fn timeout_sync_times_out() {
		time_ext::timeout_sync(
			|| {
				std::thread::sleep(Duration::from_millis(200));
				Ok(())
			},
			Duration::from_millis(10),
		)
		.unwrap_err();
	}
}
