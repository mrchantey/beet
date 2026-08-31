//! How long a raw [`AnalyticsEvent`] is kept before its store expires it.
use crate::prelude::*;
use beet_core::prelude::*;

/// The per-kind windows a raw analytics event is kept for, resolved by ancestry.
///
/// Raw per-event detail stops being useful after weeks, but the daily aggregate
/// it rolls up into is the whole point of recording at all, so the retention
/// order is fixed and this component only ever names the LAST step of it: an
/// event is archived cold and rolled up long before its window closes (see
/// [`AnalyticsRollup`]). Nothing here can shorten that order — a window below
/// [`Self::MIN_WINDOW`] would expire raws the nightly job has not covered yet,
/// so it is rejected rather than quietly honored.
///
/// One declaration covers every consumer beneath it: the router that records
/// events and the job that stamps historical ones read the same windows off the
/// same tree, rather than each carrying a copy that can drift. Absent entirely,
/// the defaults apply — requests are far higher volume and far less individually
/// interesting than the streams a client reports.
///
/// ```html
/// <Router {AnalyticsRetention{requests:"14d", events:"180d"}}>..</Router>
/// ```
///
/// A window of zero keeps that kind forever, stamping no expiry at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
pub struct AnalyticsRetention {
	/// How long an [`AnalyticsEventKind::Request`] row is kept. The raw traffic
	/// log: the highest volume stream and the one whose individual rows say the
	/// least once counted.
	pub requests: Duration,
	/// How long a client-reported row (page view, click, scroll, error) is kept.
	pub events: Duration,
}

impl Default for AnalyticsRetention {
	fn default() -> Self {
		Self {
			requests: Duration::from_secs(30 * 86_400),
			events: Duration::from_secs(90 * 86_400),
		}
	}
}

impl AnalyticsRetention {
	/// The shortest window a declaration may set, comfortably clear of the daily
	/// job's cadence so a raw row is always archived and rolled up with days to
	/// spare.
	pub const MIN_WINDOW: Duration = Duration::from_secs(7 * 86_400);

	/// How long a backfilled row already past its window is kept before
	/// expiring, so a botched sweep is observable rather than instantaneous.
	pub const GRACE: Duration = Duration::from_secs(2 * 86_400);

	/// The window for `kind`, `None` when that kind is kept forever.
	pub fn window(&self, kind: AnalyticsEventKind) -> Option<Duration> {
		match kind {
			AnalyticsEventKind::Request => self.requests,
			_ => self.events,
		}
		.xsome()
		.filter(|window| !window.is_zero())
	}

	/// The epoch second an event of `kind` recorded at `recorded` expires,
	/// `None` when that kind is kept forever.
	pub fn expires_at(
		&self,
		kind: AnalyticsEventKind,
		recorded: Duration,
	) -> Option<u64> {
		self.window(kind)
			.map(|window| (recorded + window).as_secs())
	}

	/// [`Self::expires_at`], floored at [`GRACE`](Self::GRACE) from `now`.
	///
	/// The backfill's form: a row recorded before its window opened would
	/// otherwise be stamped with an expiry already in the past, and DynamoDB
	/// would start deleting the history within the hour. The floor means a sweep
	/// that archived or rolled up the wrong thing is still recoverable when
	/// someone reads the report the next morning.
	pub fn expires_at_with_grace(
		&self,
		kind: AnalyticsEventKind,
		recorded: Duration,
		now: Duration,
	) -> Option<u64> {
		self.expires_at(kind, recorded)
			.map(|expiry| expiry.max((now + Self::GRACE).as_secs()))
	}

	/// Whether every declared window clears [`MIN_WINDOW`](Self::MIN_WINDOW),
	/// naming the one that does not.
	///
	/// Invariant: no raw event is destroyed before its aggregate and its cold
	/// archive exist. A window shorter than the job's cadence breaks that
	/// silently, days later, which is the one failure mode this whole pipeline
	/// exists to avoid.
	pub fn validate(&self) -> Result {
		for (name, window) in
			[("requests", self.requests), ("events", self.events)]
		{
			if !window.is_zero() && window < Self::MIN_WINDOW {
				bevybail!(
					"`AnalyticsRetention` keeps {name} for {}, shorter than the {} \
					 minimum: raw events must outlive the daily job that archives \
					 and rolls them up. Use `0s` to keep them forever.",
					time_ext::pretty_print_duration(window),
					time_ext::pretty_print_duration(Self::MIN_WINDOW)
				);
			}
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	const DAY: u64 = 86_400;

	/// Requests expire sooner than the streams a client reports, and a zero
	/// window keeps its kind forever.
	#[beet_core::test]
	fn stamps_per_kind_windows() {
		let retention = AnalyticsRetention::default();
		let recorded = Duration::from_secs(1_000_000);
		retention
			.expires_at(AnalyticsEventKind::Request, recorded)
			.xpect_eq(Some(1_000_000 + 30 * DAY));
		for kind in [
			AnalyticsEventKind::PageView,
			AnalyticsEventKind::Click,
			AnalyticsEventKind::Scroll,
			AnalyticsEventKind::Error,
		] {
			retention
				.expires_at(kind, recorded)
				.xpect_eq(Some(1_000_000 + 90 * DAY));
		}
		AnalyticsRetention {
			requests: Duration::ZERO,
			events: Duration::ZERO,
		}
		.expires_at(AnalyticsEventKind::Request, recorded)
		.xpect_none();
	}

	/// A backfilled row already past its window is stamped a grace period out,
	/// never an expiry in the past: a botched sweep must be readable the next
	/// morning rather than already deleted.
	#[beet_core::test]
	fn a_backfilled_row_gets_its_grace() {
		let retention = AnalyticsRetention::default();
		let now = Duration::from_secs(400 * DAY);
		// recorded a year ago, ie long past its 30 day window
		retention
			.expires_at_with_grace(
				AnalyticsEventKind::Request,
				Duration::from_secs(35 * DAY),
				now,
			)
			.xpect_eq(Some((400 + 2) * DAY));
		// a recent row keeps its own expiry, which is further out than the grace
		retention
			.expires_at_with_grace(AnalyticsEventKind::Request, now, now)
			.xpect_eq(Some((400 + 30) * DAY));
	}

	/// A window shorter than the job's cadence would expire raws nothing has
	/// archived yet, so it fails loudly instead.
	#[beet_core::test]
	fn rejects_a_window_the_job_cannot_cover() {
		AnalyticsRetention::default().validate().unwrap();
		AnalyticsRetention {
			requests: Duration::ZERO,
			events: Duration::ZERO,
		}
		.validate()
		.unwrap();
		AnalyticsRetention {
			requests: Duration::from_secs(DAY),
			..default()
		}
		.validate()
		.unwrap_err()
		.to_string()
		.xpect_contains("keeps requests for");
	}
}
