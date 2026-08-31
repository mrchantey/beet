//! The daily aggregate: what a day of raw events reduces to, and the only part
//! of it that outlives them.
use super::analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// What one [`AnalyticsRollup`] row covers on its day.
///
/// Two scopes rather than one because distinct-session counts do not sum: one
/// visit reads many pages, so adding up the per-path figures counts that visit
/// once per page. The site-wide row is the only one whose `visits` answers "how
/// many people came", and the only one a weekly or monthly figure may sum.
#[derive(
	Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum AnalyticsScope {
	/// Every event of that day, whatever path it touched.
	Site,
	/// One requested or viewed path.
	Path(SmolStr),
}

impl AnalyticsScope {
	/// The stable key this scope contributes to a row id, prefixed so a path
	/// literally named `site` cannot collide with the site-wide row.
	fn id_key(&self) -> String {
		match self {
			Self::Site => "site".into(),
			Self::Path(path) => format!("path:{path}"),
		}
	}
}

impl core::fmt::Display for AnalyticsScope {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Site => write!(f, "(whole site)"),
			Self::Path(path) => write!(f, "{path}"),
		}
	}
}

/// One day of [`AnalyticsEvent`]s reduced to counts, for one
/// [`AnalyticsScope`].
///
/// Aggregates are forever and raw events are not: the raws are archived cold and
/// then expired by the table's TTL, so everything a long-range report can ever
/// say about a past day has to already be in this row. That is why it carries
/// distributions rather than means (a mean over expired rows cannot be
/// recombined, and a dwell mean is a lie anyway — see [`Buckets::DWELL`]) and
/// why it carries a [`version`](Self::version).
///
/// The [`id`](Self::id) is a pure function of the version, date and scope, so
/// re-running a day overwrites its rows in place and a backfill is safe to run
/// twice. A rollup never carries a `ttl` attribute: the aggregates outlive the
/// events they were computed from, which is the whole point of computing them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsRollup {
	/// The deterministic row id, composed by [`Self::row_id`].
	pub id: Uuid,
	/// The schema this row was written against, so a reader can tell a row it
	/// understands from one a later beet wrote. Aggregates outlive their first
	/// schema by construction.
	pub version: u32,
	/// The UTC day covered, `YYYY-MM-DD`.
	pub date: SmolStr,
	/// What the counts below are scoped to.
	pub scope: AnalyticsScope,
	/// Page views: a viewed page, not a served request.
	pub views: u32,
	/// Distinct sessions, ie unique VISITS. A session id is a random uuid minted
	/// once per browser session and never linked across visits, so this counts
	/// visits and never visitors; that is what keeps the pipeline consent-free.
	pub visits: u32,
	/// Routed server requests, ie the raw traffic log.
	pub requests: u32,
	/// Clicks on interactive elements.
	pub clicks: u32,
	/// Client-side errors reported.
	pub errors: u32,
	/// Event counts per ISO 3166-1 alpha-2 country code.
	pub by_country: Vec<(SmolStr, u32)>,
	/// Counts per referring url, across page views and requests alike.
	pub by_referrer: Vec<(SmolStr, u32)>,
	/// Event counts per coarse client kind.
	pub by_client_kind: Vec<(ClientKind, u32)>,
	/// Request counts per status class, ie `2xx` / `4xx`.
	pub by_status_class: Vec<(SmolStr, u32)>,
	/// The pages that 404'd for a client that was *linked* there, counted per
	/// referring url: the broken links, and where they are linked from. A
	/// [`Path`](AnalyticsScope::Path) row names the missing page and this names
	/// who points at it, so the report survives the raw window.
	pub broken_links: Vec<(SmolStr, u32)>,
	/// The page-view dwell distribution, bucketed by [`Buckets::DWELL`].
	pub dwell: Buckets,
	/// The max-scroll-depth distribution, bucketed by [`Buckets::SCROLL`].
	pub scroll: Buckets,
}

impl AnalyticsRollup {
	/// The current [`version`](Self::version): bump it whenever the meaning of a
	/// field changes, never when one is added.
	pub const VERSION: u32 = 1;

	/// The namespace every aggregate row id derives from, so an id is a pure
	/// function of what it summarizes and re-running a day overwrites it.
	const NAMESPACE: Uuid =
		Uuid::from_u128(0x0192_f8a0_beef_5a11_9a11_a11a_7ce5_0011);

	/// The row id for `date` and `scope` under the current schema.
	pub fn row_id(date: &str, scope: &AnalyticsScope) -> Uuid {
		Uuid::new_v5(
			&Self::NAMESPACE,
			format!("{}:{date}:{}", Self::VERSION, scope.id_key()).as_bytes(),
		)
	}

	/// A zeroed row for `date` and `scope`.
	pub fn new(date: impl Into<SmolStr>, scope: AnalyticsScope) -> Self {
		let date = date.into();
		Self {
			id: Self::row_id(&date, &scope),
			version: Self::VERSION,
			date,
			scope,
			views: 0,
			visits: 0,
			requests: 0,
			clicks: 0,
			errors: 0,
			by_country: Vec::new(),
			by_referrer: Vec::new(),
			by_client_kind: Vec::new(),
			by_status_class: Vec::new(),
			broken_links: Vec::new(),
			dwell: Buckets::new(&Buckets::DWELL),
			scroll: Buckets::new(&Buckets::SCROLL),
		}
	}

	/// Reduce raw events into one row per (day, scope), ordered by date then
	/// scope so a run's output is a pure function of its input.
	///
	/// Every event contributes twice: to its path's row and to that day's
	/// site-wide row. The day is the UTC date of the event's server timestamp.
	pub fn from_events(events: &[AnalyticsEvent]) -> Vec<Self> {
		let mut days =
			HashMap::<(SmolStr, AnalyticsScope), Accumulator>::default();
		for event in events {
			let date = event.date();
			for scope in [
				AnalyticsScope::Site,
				AnalyticsScope::Path(event.path.clone()),
			] {
				days.entry((date.clone(), scope)).or_default().add(event);
			}
		}
		let mut rows = days
			.into_iter()
			.map(|((date, scope), acc)| acc.into_rollup(date, scope))
			.collect::<Vec<_>>();
		rows.sort_by(|left, right| {
			(&left.date, &left.scope).cmp(&(&right.date, &right.scope))
		});
		rows
	}

	/// The dates covered by `events`, ie the days a run must roll up.
	pub fn dates_of(events: &[AnalyticsEvent]) -> Vec<SmolStr> {
		let mut dates =
			events.iter().map(AnalyticsEvent::date).collect::<Vec<_>>();
		dates.sort();
		dates.dedup();
		dates
	}
}

/// The primary key is the deterministic [`row_id`](AnalyticsRollup::row_id), a
/// name-based uuid rather than a time-based one, so
/// [`timestamp`](TableStoreRow::timestamp) reads the covered day off the row
/// rather than decoding an id that embeds no clock.
impl TableStoreRow for AnalyticsRollup {
	fn id(&self) -> Uuid { self.id }
	fn timestamp(&self) -> Timestamp {
		time_ext::parse_date(&self.date)
			.unwrap_or_default()
			.xmap(Timestamp::from_unix_epoch_elapsed)
	}
}

/// The mutable half of a rollup: the count maps and the session set a row is
/// reduced from, none of which survive into the stored row.
#[derive(Default)]
struct Accumulator {
	sessions: HashSet<Uuid>,
	views: u32,
	requests: u32,
	clicks: u32,
	errors: u32,
	by_country: HashMap<SmolStr, u32>,
	by_referrer: HashMap<SmolStr, u32>,
	by_client_kind: HashMap<ClientKind, u32>,
	by_status_class: HashMap<SmolStr, u32>,
	broken_links: HashMap<SmolStr, u32>,
	dwell: Buckets,
	scroll: Buckets,
}

impl Accumulator {
	/// Fold one event in, whatever its kind.
	fn add(&mut self, event: &AnalyticsEvent) {
		*self.by_client_kind.entry(event.client_kind).or_default() += 1;
		if let Some(country) = &event.country {
			*self.by_country.entry(country.clone()).or_default() += 1;
		}
		if let Some(session) = event.session {
			self.sessions.insert(session);
		}
		match &event.data {
			AnalyticsEventData::PageView {
				duration_ms,
				referrer,
				..
			} => {
				self.views += 1;
				// a view whose closing beacon never arrived still carries its
				// last heartbeat duration, which is an honest lower bound.
				self.dwell.add(&Buckets::DWELL, *duration_ms);
				self.count_referrer(referrer);
			}
			AnalyticsEventData::Request {
				status, referrer, ..
			} => {
				self.requests += 1;
				*self
					.by_status_class
					.entry(analytics_ext::status_class(*status))
					.or_default() += 1;
				self.count_referrer(referrer);
				// a referred 404 is a link that should have worked; an
				// unreferred one is a probe the middleware already drops.
				if *status == 404
					&& let Some(referrer) = referrer
				{
					*self.broken_links.entry(referrer.clone()).or_default() +=
						1;
				}
			}
			AnalyticsEventData::Scroll { max_percent } => {
				self.scroll.add(&Buckets::SCROLL, *max_percent as u64);
			}
			AnalyticsEventData::Click { .. } => self.clicks += 1,
			AnalyticsEventData::Error { .. } => self.errors += 1,
		}
	}

	fn count_referrer(&mut self, referrer: &Option<SmolStr>) {
		if let Some(referrer) = referrer {
			*self.by_referrer.entry(referrer.clone()).or_default() += 1;
		}
	}

	/// Freeze into the stored row, the session set collapsing to its size.
	fn into_rollup(
		self,
		date: SmolStr,
		scope: AnalyticsScope,
	) -> AnalyticsRollup {
		AnalyticsRollup {
			views: self.views,
			visits: self.sessions.len() as u32,
			requests: self.requests,
			clicks: self.clicks,
			errors: self.errors,
			by_country: analytics_ext::sort_desc(self.by_country),
			by_referrer: analytics_ext::sort_desc(self.by_referrer),
			by_client_kind: analytics_ext::sort_desc(self.by_client_kind),
			by_status_class: analytics_ext::sort_desc(self.by_status_class),
			broken_links: analytics_ext::sort_desc(self.broken_links),
			dwell: self.dwell,
			scroll: self.scroll,
			..AnalyticsRollup::new(date, scope)
		}
	}
}

/// The fixed upper edges one kind of distribution is bucketed by, and the label
/// each bucket reads as.
///
/// A `'static` scale rather than a field of every [`Buckets`]: the edges are a
/// property of what is being measured, not of any one day's counts, and storing
/// them per row would be the same six numbers on every aggregate forever.
pub struct BucketScale {
	/// The exclusive upper edge of each bucket. The last one is the cap: a value
	/// beyond it counts in that bucket rather than being dropped or extending
	/// the scale.
	pub edges: &'static [u64],
	/// One human label per edge, for reports.
	pub labels: &'static [&'static str],
}

/// A count per bucket of a [`BucketScale`], ie what an aggregate keeps instead
/// of a mean.
///
/// A mean cannot be recombined once the rows it averaged have expired, and for
/// dwell it is not even meaningful before then: heartbeat-accumulating tabs and
/// views whose closing beacon never arrived drag it into the hours. A
/// distribution answers the same question honestly and survives the raws.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Buckets {
	/// One count per [`BucketScale::edges`] entry.
	counts: Vec<u32>,
}

impl Buckets {
	/// Page-view dwell, in milliseconds. The top bucket caps everything beyond
	/// it, so a tab left open overnight cannot skew the distribution.
	pub const DWELL: BucketScale = BucketScale {
		edges: &[10_000, 30_000, 60_000, 180_000, 600_000, 1_800_000],
		labels: &["0-10s", "10-30s", "30-60s", "1-3m", "3-10m", "10-30m+"],
	};

	/// Max scroll depth reached, as a percentage of the page.
	pub const SCROLL: BucketScale = BucketScale {
		edges: &[25, 50, 75, 100],
		labels: &["0-25%", "25-50%", "50-75%", "75-100%"],
	};

	/// An empty distribution over `scale`.
	pub fn new(scale: &BucketScale) -> Self {
		Self {
			counts: vec![0; scale.edges.len()],
		}
	}

	/// Count `value` in its bucket, the last bucket capping everything beyond
	/// the final edge.
	pub fn add(&mut self, scale: &BucketScale, value: u64) {
		if self.counts.len() != scale.edges.len() {
			self.counts.resize(scale.edges.len(), 0);
		}
		let index = scale
			.edges
			.iter()
			.position(|edge| value < *edge)
			.unwrap_or(scale.edges.len() - 1);
		self.counts[index] += 1;
	}

	/// Fold another distribution over the same scale into this one, ie to derive
	/// a week from its days.
	pub fn merge(&mut self, other: &Self) {
		if self.counts.len() < other.counts.len() {
			self.counts.resize(other.counts.len(), 0);
		}
		for (index, count) in other.counts.iter().enumerate() {
			self.counts[index] += count;
		}
	}

	/// Total values counted.
	pub fn total(&self) -> u32 { self.counts.iter().sum() }

	/// Each bucket's label and count, for a report.
	pub fn rows(&self, scale: &BucketScale) -> Vec<(&'static str, u32)> {
		scale
			.labels
			.iter()
			.copied()
			.zip(self.counts.iter().copied())
			.collect()
	}

	/// The label of the bucket the `percentile`-th value falls in, ie
	/// `p(&Buckets::DWELL, 0.5)` for the median. `None` when nothing was
	/// counted.
	///
	/// A bucket rather than a number: the counts are all there is, so any
	/// interpolated figure would be precision this distribution does not have.
	pub fn percentile(
		&self,
		scale: &BucketScale,
		percentile: f64,
	) -> Option<&'static str> {
		let total = self.total();
		if total == 0 {
			return None;
		}
		// the index of the value at `percentile`, ie 0 for the first
		let target = ((total as f64 * percentile).ceil() as u32).max(1);
		let mut seen = 0;
		scale.labels.iter().zip(self.counts.iter()).find_map(
			|(label, count)| {
				seen += count;
				(seen >= target).then_some(*label)
			},
		)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A day's epoch milliseconds, `hour` into the UTC day.
	fn at(date: &str, hour: u64) -> u64 {
		(time_ext::parse_date(date).unwrap().as_secs() + hour * 3600) * 1000
	}

	/// An event of `data` at `path` on `date`, attributed to `session`.
	fn event(
		date: &str,
		path: &str,
		session: u128,
		data: AnalyticsEventData,
	) -> AnalyticsEvent {
		let mut event = AnalyticsEvent::new(path, data)
			.with_client_kind(ClientKind::Web)
			.with_session(Some(Uuid::from_u128(session)));
		event.timestamp = at(date, 12);
		event.country = Some("AU".into());
		event
	}

	fn page_view(
		date: &str,
		path: &str,
		session: u128,
		duration_ms: u64,
	) -> AnalyticsEvent {
		event(date, path, session, AnalyticsEventData::PageView {
			duration_ms,
			referrer: None,
			title: None,
			client: default(),
		})
	}

	fn request(
		date: &str,
		path: &str,
		status: u16,
		referrer: Option<&str>,
	) -> AnalyticsEvent {
		event(date, path, 1, AnalyticsEventData::Request {
			status,
			method: "Get".into(),
			user_agent: None,
			referrer: referrer.map(Into::into),
		})
	}

	/// The row for `date` and `scope`, which must exist.
	fn row(
		rows: &[AnalyticsRollup],
		date: &str,
		scope: AnalyticsScope,
	) -> AnalyticsRollup {
		rows.iter()
			.find(|row| row.date == date && row.scope == scope)
			.unwrap()
			.clone()
	}

	/// A multi-day, multi-path day reduces to one row per path plus one for the
	/// site, with every column scoped to its own row.
	#[beet_core::test]
	fn rolls_up_days_and_paths() {
		let rows = AnalyticsRollup::from_events(&[
			page_view("2026-08-01", "/", 1, 12_000),
			page_view("2026-08-01", "/docs", 1, 45_000),
			page_view("2026-08-01", "/docs", 2, 5_000),
			request("2026-08-01", "/docs", 200, None),
			request("2026-08-01", "/docs/typo", 404, Some("https://beet.org/")),
			page_view("2026-08-02", "/", 3, 1_000),
		]);
		// 2 days: (site, /, /docs, /docs/typo) then (site, /)
		rows.len().xpect_eq(6);

		let docs =
			row(&rows, "2026-08-01", AnalyticsScope::Path("/docs".into()));
		docs.views.xpect_eq(2);
		docs.visits.xpect_eq(2);
		docs.requests.xpect_eq(1);
		docs.by_status_class.xpect_eq(vec![("2xx".into(), 1)]);
		docs.by_country.xpect_eq(vec![("AU".into(), 3)]);
		docs.by_client_kind.xpect_eq(vec![(ClientKind::Web, 3)]);

		// the broken link is named by the page that 404'd and counted by who
		// links to it
		row(
			&rows,
			"2026-08-01",
			AnalyticsScope::Path("/docs/typo".into()),
		)
		.broken_links
		.xpect_eq(vec![("https://beet.org/".into(), 1)]);

		// the site row is the whole day, and its visits DEDUPE the session that
		// read two paths: per-path distincts do not sum.
		let site = row(&rows, "2026-08-01", AnalyticsScope::Site);
		site.views.xpect_eq(3);
		site.requests.xpect_eq(2);
		site.visits.xpect_eq(2);
		// the second day is its own set of rows
		row(&rows, "2026-08-02", AnalyticsScope::Site)
			.views
			.xpect_eq(1);
		AnalyticsRollup::dates_of(&[
			page_view("2026-08-02", "/", 1, 0),
			page_view("2026-08-01", "/", 1, 0),
			page_view("2026-08-01", "/", 1, 0),
		])
		.xpect_eq(vec![
			SmolStr::from("2026-08-01"),
			SmolStr::from("2026-08-02"),
		]);
	}

	/// Re-running a day writes the same rows: the id is a pure function of the
	/// version, date and scope, so a backfill run twice overwrites rather than
	/// duplicating.
	#[beet_core::test]
	fn rerunning_a_day_is_idempotent() {
		let events = [
			page_view("2026-08-01", "/", 1, 12_000),
			request("2026-08-01", "/", 200, None),
		];
		AnalyticsRollup::from_events(&events)
			.xpect_eq(AnalyticsRollup::from_events(&events));
		AnalyticsRollup::row_id("2026-08-01", &AnalyticsScope::Site)
			.xpect_not_eq(AnalyticsRollup::row_id(
				"2026-08-02",
				&AnalyticsScope::Site,
			));
		// a path literally named `site` cannot collide with the site-wide row
		AnalyticsRollup::row_id("2026-08-01", &AnalyticsScope::Site)
			.xpect_not_eq(AnalyticsRollup::row_id(
				"2026-08-01",
				&AnalyticsScope::Path("site".into()),
			));
		// every row carries the schema it was written against, and NO row carries
		// an expiry: the aggregates outlive the events they were reduced from,
		// which is the whole reason for reducing them.
		for row in AnalyticsRollup::from_events(&events) {
			row.version.xpect_eq(AnalyticsRollup::VERSION);
			serde_json::to_string(&row)
				.unwrap()
				.as_str()
				.xnot()
				.xpect_contains("ttl");
		}
	}

	/// Dwell is a distribution, and the top bucket caps: an overnight tab lands
	/// beside a ten minute read rather than dragging a mean into the hours. A
	/// heartbeat-only view (its closing beacon lost) counts at its last known
	/// duration, which is an honest lower bound.
	#[beet_core::test]
	fn buckets_dwell_and_caps_the_tail() {
		let rows = AnalyticsRollup::from_events(&[
			page_view("2026-08-01", "/", 1, 3_000),
			page_view("2026-08-01", "/", 2, 20_000),
			page_view("2026-08-01", "/", 3, 20_000),
			// a heartbeat-only view: 10s of accumulated heartbeats
			page_view("2026-08-01", "/", 4, 10_000),
			// an overnight tab, capped into the top bucket
			page_view("2026-08-01", "/", 5, 13 * 60 * 60 * 1000),
		]);
		let dwell = row(&rows, "2026-08-01", AnalyticsScope::Site).dwell;
		dwell.total().xpect_eq(5);
		dwell.rows(&Buckets::DWELL).xpect_eq(vec![
			("0-10s", 1),
			("10-30s", 3),
			("30-60s", 0),
			("1-3m", 0),
			("3-10m", 0),
			("10-30m+", 1),
		]);
		dwell
			.percentile(&Buckets::DWELL, 0.5)
			.xpect_eq(Some("10-30s"));
		dwell
			.percentile(&Buckets::DWELL, 0.9)
			.xpect_eq(Some("10-30m+"));
		// an empty distribution has no percentile rather than a zero
		Buckets::new(&Buckets::DWELL)
			.percentile(&Buckets::DWELL, 0.5)
			.xpect_none();
	}

	/// Scroll depth buckets the same way, and merging days sums bucket-wise so a
	/// week derives from its days without the raws.
	#[beet_core::test]
	fn buckets_scroll_and_merges() {
		let rows = AnalyticsRollup::from_events(&[
			event("2026-08-01", "/", 1, AnalyticsEventData::Scroll {
				max_percent: 10,
			}),
			event("2026-08-01", "/", 2, AnalyticsEventData::Scroll {
				max_percent: 100,
			}),
			event("2026-08-02", "/", 3, AnalyticsEventData::Scroll {
				max_percent: 80,
			}),
		]);
		let mut week = row(&rows, "2026-08-01", AnalyticsScope::Site).scroll;
		week.merge(&row(&rows, "2026-08-02", AnalyticsScope::Site).scroll);
		week.rows(&Buckets::SCROLL).xpect_eq(vec![
			("0-25%", 1),
			("25-50%", 0),
			("50-75%", 0),
			("75-100%", 2),
		]);
	}
}
