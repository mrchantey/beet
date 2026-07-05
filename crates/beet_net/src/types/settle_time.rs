//! [`SettleTime`]: how long a caller should wait for a commanded effect to settle.
use beet_core::prelude::*;

/// A capability reply telling the caller how long to wait for the commanded
/// effect to *settle* — a drive step to finish, a spoken line to end — before
/// treating the action as complete.
///
/// Returned by a body that cannot block on its own effect (eg an esp robot whose
/// handler task has no async-timer waker): it replies with the settle budget and
/// the caller waits it out, so the next command still follows a finished effect.
/// [`None`] means no wait — the effect is already done, or there is nothing to
/// settle. Shared upstream so agent and body speak one wire type rather than
/// each duplicating a `settle` reply.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SettleTime {
	/// How long to wait for the effect to settle, or [`None`] for no wait.
	pub duration: Option<Duration>,
}

impl SettleTime {
	/// A reply with nothing to wait for.
	pub const NONE: Self = Self { duration: None };

	/// A reply asking the caller to wait `duration` for the effect to settle.
	pub fn new(duration: Duration) -> Self {
		Self {
			duration: Some(duration),
		}
	}

	/// The wait, or [`Duration::ZERO`] when there is nothing to settle.
	pub fn duration_or_zero(&self) -> Duration {
		self.duration.unwrap_or(Duration::ZERO)
	}
}
