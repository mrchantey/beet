use crate::prelude::Duration;
use extend::ext;

/// Extension trait for [`Duration`] providing convenient constructors.
#[ext]
pub impl Duration {
	/// Creates a duration from hours, minutes, and seconds.
	///
	/// # Examples
	///
	/// ```
	/// use beet_core::prelude::*;
	///
	/// let duration = Duration::from_hms(1, 30, 45);
	/// assert_eq!(duration.as_secs(), 5445); // 1*3600 + 30*60 + 45
	/// ```
	fn from_hms(hour: u64, minute: u64, second: u64) -> Duration {
		Duration::from_secs(hour * 3600 + minute * 60 + second)
	}

	/// Parse a human duration like `"50ms"`, `"1.5s"`, `"2m"`, `"1h"` or `"7d"`. The
	/// unit (`ns`, `us`/`µs`, `ms`, `s`, `m`, `h`, `d`) is required; returns `None` on a
	/// missing or unknown unit, so a bare number is never silently assumed to be seconds.
	/// Shared by markup attribute coercion and the drive-command wire serde.
	fn from_human_str(string: &str) -> Option<Duration> {
		let string = string.trim();
		let split = string
			.find(|c: char| !c.is_ascii_digit() && c != '.')
			.unwrap_or(string.len());
		let (number, unit) = string.split_at(split);
		let number: f64 = number.parse().ok()?;
		let secs = match unit.trim() {
			"ns" => number / 1_000_000_000.0,
			"us" | "µs" => number / 1_000_000.0,
			"ms" => number / 1_000.0,
			"s" => number,
			"m" => number * 60.0,
			"h" => number * 60.0 * 60.0,
			"d" => number * 24.0 * 60.0 * 60.0,
			_ => return None,
		};
		Some(Duration::from_secs_f64(secs))
	}
}
