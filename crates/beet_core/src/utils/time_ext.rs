//! Time utilities for cross-platform duration handling and async sleep.
//!
//! The wall-clock surface ([`now`]/[`now_millis`]/[`set_now`]) and the sleep
//! helpers are no_std ([`sleep`] on a bare target rides a [`set_sleep`] hook);
//! only [`timeout_sync`] is std-only.

use crate::prelude::*;
use bevy::platform::sync::OnceLock;
use core::time::Duration;

/// A wall-clock source: the current time as a [`Duration`] since the Unix epoch.
///
/// This is the no_std-friendly clock hook, mirroring `Instant::set_elapsed` (for
/// the monotonic clock) and `Request::set_http_client` (for transport). Installed via
/// [`set_now`] to override the platform default, eg so a bare target's SNTP
/// client can supply wall-clock time once it has synced.
pub type NowFn = fn() -> Duration;

static NOW: OnceLock<NowFn> = OnceLock::new();

/// Install the wall-clock source used by [`now`] and [`try_now`].
///
/// Call once the source is ready (eg after an SNTP sync). Takes precedence over
/// the platform default, so it can also be used to mock time. Returns an error
/// if a source has already been installed.
pub fn set_now(getter: NowFn) -> Result {
	NOW.set(getter)
		.map_err(|_| bevyhow!("a `now` clock source is already installed"))
}

/// The current time as a [`Duration`] since the Unix epoch, or an error if no
/// clock is available yet.
///
/// Resolution order:
/// 1. a source installed via [`set_now`];
/// 2. the platform default: `SystemTime` on std native, `Date::now()` on wasm;
/// 3. otherwise an error — eg a bare target whose SNTP client hasn't synced.
///
/// Prefer this over [`now`] when the clock may still be loading.
pub fn try_now() -> Result<Duration> {
	if let Some(getter) = NOW.get() {
		return Ok(getter());
	}
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			Ok(Duration::from_millis(js_sys::Date::now() as u64))
		} else if #[cfg(feature = "std")] {
			std::time::SystemTime::now()
				.duration_since(std::time::UNIX_EPOCH)
				.map_err(|err| bevyhow!("system clock is before the Unix epoch: {err}"))
		} else {
			bevybail!(
				"no wall clock installed: call `set_now` at boot \
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

/// The current year, derived from the cross-platform [`time_ext::now`]
/// wall clock (seconds since the Unix epoch). Approximation is fine for a
/// footer string (off by at most a day around new year).
pub fn current_year() -> i32 {
	let secs = time_ext::now().as_secs();
	1970 + (secs as f64 / (365.2425 * 86400.0)) as i32
}

/// The UTC civil `(year, month, day)` of a unix epoch [`Duration`].
///
/// The days-to-civil algorithm (Howard Hinnant) directly rather than a datetime
/// crate; epoch durations are unsigned so pre-1970 dates cannot occur.
pub fn civil_date(unix: Duration) -> (u64, u64, u64) {
	let days = unix.as_secs() / 86_400 + 719_468;
	let era = days / 146_097;
	let doe = days - era * 146_097;
	let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
	let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
	let mp = (5 * doy + 2) / 153;
	let day = doy - (153 * mp + 2) / 5 + 1;
	let month = if mp < 10 { mp + 3 } else { mp - 9 };
	(yoe + era * 400 + (month <= 2) as u64, month, day)
}

/// Formats a unix epoch [`Duration`] as a `YYYY-MM-DD` UTC date, ie the key a
/// daily aggregate row or archive object is named by.
pub fn format_date(unix: Duration) -> String {
	let (year, month, day) = civil_date(unix);
	format!("{year:04}-{month:02}-{day:02}")
}

/// The English name of a month, `1..=12`. `None` outside that range.
pub fn month_name(month: u64) -> Option<&'static str> {
	MONTH_NAMES.get(month.checked_sub(1)? as usize).copied()
}

/// The month names [`month_name`] indexes, January first.
const MONTH_NAMES: [&str; 12] = [
	"January",
	"February",
	"March",
	"April",
	"May",
	"June",
	"July",
	"August",
	"September",
	"October",
	"November",
	"December",
];

/// Formats a unix epoch [`Duration`] as a UTC date for a reader rather than a
/// key, eg `6 September 2025`.
///
/// Day-month-year with the month spelled out: unambiguous in every locale (where
/// `06/09/2025` is not), and free of the ordinal suffix and comma that make
/// `6th September, 2025` read like handwriting.
pub fn format_long_date(unix: Duration) -> String {
	let (year, month, day) = civil_date(unix);
	match month_name(month) {
		Some(name) => format!("{day} {name} {year}"),
		// unreachable via `civil_date`, whose month is always 1..=12
		None => format_date(unix),
	}
}

/// The unix epoch [`Duration`] of midnight UTC on a `YYYY-MM-DD` date, the
/// inverse of [`format_date`]. `None` on anything that is not that shape.
///
/// The civil-to-days half of the algorithm [`civil_date`] inverts, so a stored
/// date key (an aggregate row, an archive object) reads back as an instant
/// without a datetime crate.
pub fn parse_date(date: &str) -> Option<Duration> {
	let mut parts = date.split('-');
	let (year, month, day) = (
		parts.next()?.parse::<u64>().ok()?,
		parts.next()?.parse::<u64>().ok()?,
		parts.next()?.parse::<u64>().ok()?,
	);
	if parts.next().is_some() || !(1..=12).contains(&month) || day == 0 {
		return None;
	}
	// civil-to-days (Howard Hinnant), the year shifted so march leads the year
	let year = year - (month <= 2) as u64;
	let era = year / 400;
	let yoe = year - era * 400;
	let mp = if month > 2 { month - 3 } else { month + 9 };
	let doy = (153 * mp + 2) / 5 + day - 1;
	let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
	// epoch durations are unsigned, so a pre-1970 date has no representation
	(era * 146_097 + doe)
		.checked_sub(719_468)
		.map(|days| Duration::from_secs(days * 86_400))
}

/// Formats a unix epoch [`Duration`] as an ISO 8601 / RFC 3339 UTC timestamp
/// with millisecond precision, eg `2024-09-09T19:46:02.102Z`.
pub fn format_iso8601(unix: Duration) -> String {
	let secs = unix.as_secs();
	let millis = unix.subsec_millis();
	let (hour, min, sec) = ((secs / 3600) % 24, (secs / 60) % 60, secs % 60);
	format!(
		"{}T{hour:02}:{min:02}:{sec:02}.{millis:03}Z",
		format_date(unix)
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

/// Milliseconds since the Unix epoch (`now().as_millis()`).
pub fn now_millis() -> u128 { now().as_millis() }

/// An async sleep source: a future resolving after the given [`Duration`].
///
/// The no_std-friendly sleep hook, mirroring [`NowFn`] (wall clock) and
/// `Request::set_http_client` (transport). Installed via [`set_sleep`] so a bare target
/// can back [`sleep`] with its own timer, eg an `embassy_time::Timer`. The
/// future is `Send + Sync` (not `MaybeSend`) because [`sleep`] is awaited
/// inside `Send + Sync` values (eg an HTTP body stream, whose `Request`/
/// `Response` must be `Sync` to serve as an action input) even in
/// single-threaded builds.
pub type SleepFn = fn(Duration) -> SendSyncBoxedFuture<()>;

static SLEEP: OnceLock<SleepFn> = OnceLock::new();

/// Install the sleep source used by [`sleep`].
///
/// Takes precedence over the platform default, so it can also be used to mock
/// time. Returns an error if a source has already been installed.
pub fn set_sleep(sleeper: SleepFn) -> Result {
	SLEEP
		.set(sleeper)
		.map_err(|_| bevyhow!("a `sleep` source is already installed"))
}

/// Sleeps for the specified number of seconds.
pub async fn sleep_secs(secs: u64) { sleep(Duration::from_secs(secs)).await; }

/// Sleeps for the specified number of milliseconds.
pub async fn sleep_millis(millis: u64) {
	sleep(Duration::from_millis(millis)).await;
}

/// Sleeps for the specified number of microseconds.
pub async fn sleep_micros(micros: u64) {
	sleep(Duration::from_micros(micros)).await;
}

/// Cross platform sleep function
///
/// Resolution order:
/// 1. a source installed via [`set_sleep`];
/// 2. the platform default: `setTimeout` on wasm, `async-io` on std native;
/// 3. otherwise a panic — a bare target installs a source at boot.
pub async fn sleep(duration: Duration) {
	if let Some(sleeper) = SLEEP.get() {
		return sleeper(duration).await;
	}
	cfg_if! {
		if #[cfg(target_arch = "wasm32")] {
			use wasm_bindgen::JsCast;
			use wasm_bindgen::JsValue;
			use wasm_bindgen_futures::JsFuture;
			// resolve the *global* `setTimeout` by reflection rather than reaching
			// through `web_sys::window()`: a Cloudflare Worker (and other non-DOM
			// hosts) has no `window`, so `window().unwrap()` would panic.
			// `setTimeout` lives on `globalThis` in every JS host (browser, worker,
			// deno). Reflection (vs a `#[wasm_bindgen] extern`) also avoids a
			// duplicate `__wbindgen_describe___wbg_setTimeout` symbol when a `--lib`
			// wasm test links this crate twice (cfg(test) + rlib).
			let set_timeout: js_sys::Function =
				js_sys::Reflect::get(&js_sys::global(), &"setTimeout".into())
					.expect("globalThis.setTimeout")
					.unchecked_into();
			let promise = js_sys::Promise::new(&mut |resolve, _| {
				set_timeout
					.call2(
						&JsValue::NULL,
						&resolve,
						&JsValue::from(duration.as_millis() as i32),
					)
					.expect("call setTimeout");
			});
			JsFuture::from(promise)
				.await
				.expect("should await `setTimeout` OK");
		} else if #[cfg(feature = "std")] {
			async_io::Timer::after(duration).await;
		} else {
			panic!("no sleep source installed: call `set_sleep` at boot (eg an embassy timer)");
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

	#[crate::test]
	fn formats_iso8601() {
		time_ext::format_iso8601(Duration::ZERO)
			.xpect_eq("1970-01-01T00:00:00.000Z");
		time_ext::format_iso8601(Duration::from_millis(1_725_911_162_102))
			.xpect_eq("2024-09-09T19:46:02.102Z");
		// leap year day
		time_ext::format_iso8601(Duration::from_secs(1_709_164_800))
			.xpect_eq("2024-02-29T00:00:00.000Z");
	}

	/// The reader-facing form: no ordinal suffix, no comma, month spelled out.
	#[crate::test]
	fn formats_long_date() {
		time_ext::format_long_date(Duration::ZERO).xpect_eq("1 January 1970");
		time_ext::format_long_date(Duration::from_secs(1_757_116_800))
			.xpect_eq("6 September 2025");
		// leap year day
		time_ext::format_long_date(Duration::from_secs(1_709_164_800))
			.xpect_eq("29 February 2024");
	}

	/// The date half alone, the key a daily aggregate or archive object takes.
	#[crate::test]
	fn formats_date() {
		time_ext::format_date(Duration::ZERO).xpect_eq("1970-01-01");
		// the last millisecond of a day still belongs to that day
		time_ext::format_date(Duration::from_millis(1_725_926_399_999))
			.xpect_eq("2024-09-09");
		time_ext::format_date(Duration::from_millis(1_725_926_400_000))
			.xpect_eq("2024-09-10");
	}

	/// A stored date key reads back as the instant it names, so the two halves
	/// of the civil algorithm agree across the whole epoch range.
	#[crate::test]
	fn parses_date() {
		for secs in (0..2_000_000_000u64).step_by(86_400 * 37) {
			let unix = Duration::from_secs(secs);
			time_ext::parse_date(&time_ext::format_date(unix))
				.unwrap()
				.xpect_eq(Duration::from_secs(secs - secs % 86_400));
		}
		time_ext::parse_date("2024-02-29")
			.unwrap()
			.xpect_eq(Duration::from_secs(1_709_164_800));
		// pre-epoch and malformed dates have no instant
		for date in ["1969-12-31", "2024-13-01", "2024-02", "not-a-date", ""] {
			time_ext::parse_date(date).xpect_none();
		}
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
