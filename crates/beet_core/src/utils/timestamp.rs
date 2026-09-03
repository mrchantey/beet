use crate::prelude::*;
use core::time::Duration;

/// A wall-clock instant, stored as the [`Duration`] elapsed since the Unix epoch.
///
/// The serializable counterpart of [`Instant`]: that clock is monotonic (elapsed
/// from an arbitrary process-local zero), so it is meaningless once written to a
/// store and read back in another process. This one is absolute, and its ordering
/// survives the round trip, which is what a persisted `created` field needs.
///
/// Reads the cross-platform [`time_ext::now`] rather than `SystemTime`, so it
/// works on wasm and no_std alike.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Reflect,
)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
pub struct Timestamp(Duration);

impl Timestamp {
	/// The current wall-clock time.
	pub fn now() -> Self { Self(time_ext::now()) }

	/// An instant `elapsed` after the Unix epoch, for a time that came from
	/// somewhere other than the clock (a decoded uuid, a parsed header).
	pub fn from_unix_epoch_elapsed(elapsed: Duration) -> Self { Self(elapsed) }

	/// Time elapsed since the Unix epoch.
	pub fn unix_epoch_elapsed(&self) -> Duration { self.0 }

	/// Midnight UTC on a `YYYY-MM-DD` date, ie the instant a date-only field
	/// (a post's `created`, an archive key) names. `None` on any other shape.
	pub fn parse_date(date: &str) -> Option<Self> {
		time_ext::parse_date(date).map(Self)
	}

	/// This instant's UTC date as `YYYY-MM-DD`, the inverse of
	/// [`parse_date`](Self::parse_date).
	pub fn format_date(&self) -> String { time_ext::format_date(self.0) }

	/// This instant's UTC date spelled for a reader, eg `6 September 2025`.
	pub fn format_long_date(&self) -> String {
		time_ext::format_long_date(self.0)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// A date-only field round-trips through the instant it names, and anything
	/// that is not a date has none.
	#[crate::test]
	fn parses_and_formats_dates() {
		let created = Timestamp::parse_date("2025-09-06").unwrap();
		created.format_date().xpect_eq("2025-09-06");
		created.format_long_date().xpect_eq("6 September 2025");
		Timestamp::parse_date("someday").xpect_none();
	}
}
