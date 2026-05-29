//! Pure duration formatting, usable on no_std (unlike the std-only clock and
//! sleep helpers in [`time_ext`](super::time_ext)).
use alloc::format;
use alloc::string::String;
use core::time::Duration;

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
