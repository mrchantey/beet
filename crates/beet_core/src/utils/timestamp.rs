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
}
