//! The [`AnalyticsSummary`] read model.
use super::analytics_ext;
use crate::prelude::*;
use beet_core::prelude::*;

/// An aggregate view over analytics: what kinds of clients connected, the pages
/// they viewed, and how long for.
///
/// The read model behind `beet analytics` and any dashboard. It composes from
/// two sources and needs neither: [`AnalyticsRollup`] rows for the long history
/// (the raws they cover are archived cold and then expired) and raw
/// [`AnalyticsEvent`]s for the recent window no aggregate covers yet. A day is
/// read from exactly one of them, so the two never double count.
#[derive(Debug, Default, Clone)]
pub struct AnalyticsSummary {
	/// Total events across all kinds.
	pub total: usize,
	/// Page-view events (a viewed page with a dwell duration).
	pub page_views: usize,
	/// Request-log events (raw server traffic).
	pub requests: usize,
	/// Distinct sessions seen, ie unique VISITS.
	///
	/// A session id is a random uuid minted once per browser session and never
	/// linked across visits, so this counts visits and never visitors. Composed
	/// from the site-wide daily figure rather than summed per path, since one
	/// visit reads many pages.
	pub visits: usize,
	/// The page-view dwell distribution ([`Buckets::DWELL`]).
	///
	/// A distribution, never a mean: heartbeat-accumulating tabs and views whose
	/// closing beacon never arrived drag a mean into the hours, and a mean over
	/// days whose raws have expired cannot be recombined at all.
	pub dwell: Buckets,
	/// The max-scroll-depth distribution ([`Buckets::SCROLL`]).
	pub scroll: Buckets,
	/// Page-view counts per path, most-visited first.
	pub by_path: Vec<(SmolStr, usize)>,
	/// Event counts per kind, most-common first.
	pub by_kind: Vec<(AnalyticsEventKind, usize)>,
	/// Event counts per client kind, most-common first.
	pub by_client_kind: Vec<(ClientKind, usize)>,
	/// Event counts per country code, most-common first.
	pub by_country: Vec<(SmolStr, usize)>,
	/// Paths that 404'd for a client that was *linked* there (the request carried
	/// a [`Referer`](crate::prelude::headers::Referer)), most-requested first:
	/// the broken links, ie the 404s someone expected to work.
	pub broken_links: Vec<(SmolStr, usize)>,
	/// Count of 404s nobody linked to, ie vulnerability probes walking a list
	/// (`/wp-login.php`, `/.env`). Counted rather than listed: on a public site
	/// they outnumber every other event and their paths are noise.
	///
	/// Only ever nonzero for events recorded elsewhere or before the router
	/// middleware started dropping them at the source, which it now does. Never
	/// carried by an aggregate, which has no such rows to count.
	pub probes: usize,
	/// The UTC days covered, ascending.
	pub dates: Vec<SmolStr>,
}

impl AnalyticsSummary {
	/// Aggregates a slice of raw events into a summary.
	pub fn from_events(events: &[AnalyticsEvent]) -> Self {
		let mut acc = Accumulator::default();
		for event in events {
			acc.add_event(event);
		}
		acc.into_summary()
	}

	/// Aggregates a slice of daily [`AnalyticsRollup`] rows into a summary, ie
	/// the long history whose raw events no longer exist.
	pub fn from_rollups(rollups: &[AnalyticsRollup]) -> Self {
		let mut acc = Accumulator::default();
		for rollup in rollups {
			acc.add_rollup(rollup);
		}
		acc.into_summary()
	}

	/// The full picture: aggregates for every day they cover, raw events for the
	/// days they do not.
	///
	/// A day appears in exactly one source. Reading the raws of an already
	/// aggregated day would count it twice, and a beet site keeps the tail of
	/// its raw window around long after the day is rolled up.
	pub fn compose(
		rollups: &[AnalyticsRollup],
		events: &[AnalyticsEvent],
	) -> Self {
		let covered = rollups
			.iter()
			.map(|rollup| rollup.date.clone())
			.collect::<HashSet<_>>();
		let mut acc = Accumulator::default();
		for rollup in rollups {
			acc.add_rollup(rollup);
		}
		for event in events
			.iter()
			.filter(|event| !covered.contains(&event.date()))
		{
			acc.add_event(event);
		}
		acc.into_summary()
	}

	/// The first and last day covered, `None` when nothing was.
	pub fn span(&self) -> Option<(&SmolStr, &SmolStr)> {
		self.dates.first().zip(self.dates.last())
	}
}

/// The mutable half of a summary: the count maps and session set both sources
/// fold into, so an aggregated day and a raw one land in one shape.
#[derive(Default)]
struct Accumulator {
	sessions: HashSet<Uuid>,
	/// Visits already counted by a site-wide aggregate row, which carries a
	/// distinct count rather than the ids behind it.
	aggregated_visits: usize,
	total: usize,
	page_views: usize,
	requests: usize,
	probes: usize,
	dwell: Buckets,
	scroll: Buckets,
	dates: HashSet<SmolStr>,
	by_path: HashMap<SmolStr, usize>,
	by_kind: HashMap<AnalyticsEventKind, usize>,
	by_client_kind: HashMap<ClientKind, usize>,
	by_country: HashMap<SmolStr, usize>,
	broken_links: HashMap<SmolStr, usize>,
}

impl Accumulator {
	fn add_event(&mut self, event: &AnalyticsEvent) {
		self.total += 1;
		self.dates.insert(event.date());
		*self.by_kind.entry(event.event_kind).or_default() += 1;
		match &event.data {
			AnalyticsEventData::PageView { duration_ms, .. } => {
				self.page_views += 1;
				*self.by_path.entry(event.path.clone()).or_default() += 1;
				self.dwell.add(&Buckets::DWELL, *duration_ms);
			}
			AnalyticsEventData::Request {
				status, referrer, ..
			} => {
				self.requests += 1;
				// a 404 splits by whether anything linked to it: a referred one
				// is a link that should have worked, a cold one is a probe.
				if *status == 404 {
					match referrer {
						Some(_) => {
							*self
								.broken_links
								.entry(event.path.clone())
								.or_default() += 1
						}
						None => self.probes += 1,
					}
				}
			}
			AnalyticsEventData::Scroll { max_percent } => {
				self.scroll.add(&Buckets::SCROLL, *max_percent as u64)
			}
			_ => {}
		}
		*self.by_client_kind.entry(event.client_kind).or_default() += 1;
		if let Some(country) = &event.country {
			*self.by_country.entry(country.clone()).or_default() += 1;
		}
		if let Some(session) = event.session {
			self.sessions.insert(session);
		}
	}

	/// Fold in one aggregate row.
	///
	/// The two scopes carry different halves of the day and adding both would
	/// double every column: a [`Site`](AnalyticsScope::Site) row IS the day, and
	/// a [`Path`](AnalyticsScope::Path) row only contributes what is per-path.
	fn add_rollup(&mut self, rollup: &AnalyticsRollup) {
		self.dates.insert(rollup.date.clone());
		match &rollup.scope {
			AnalyticsScope::Path(path) => {
				// a path with no views (a 404, an asset) has no entry rather
				// than a zero, matching what the raws produce
				if rollup.views > 0 {
					*self.by_path.entry(path.clone()).or_default() +=
						rollup.views as usize;
				}
				// the aggregate names the referrers linking to a missing page;
				// the summary reports the page they point at
				let broken = rollup
					.broken_links
					.iter()
					.map(|(_, count)| *count as usize)
					.sum::<usize>();
				if broken > 0 {
					*self.broken_links.entry(path.clone()).or_default() +=
						broken;
				}
			}
			AnalyticsScope::Site => {
				let clicks = rollup.clicks as usize;
				let errors = rollup.errors as usize;
				let scrolls = rollup.scroll.total() as usize;
				self.page_views += rollup.views as usize;
				self.requests += rollup.requests as usize;
				self.total += rollup.views as usize
					+ rollup.requests as usize
					+ clicks + errors
					+ scrolls;
				// the distinct count is all an aggregate keeps, so visits are
				// summed across days rather than deduped: a visit spanning
				// midnight counts twice, which is accepted.
				self.aggregated_visits += rollup.visits as usize;
				self.dwell.merge(&rollup.dwell);
				self.scroll.merge(&rollup.scroll);
				for (kind, count) in [
					(AnalyticsEventKind::PageView, rollup.views as usize),
					(AnalyticsEventKind::Request, rollup.requests as usize),
					(AnalyticsEventKind::Click, clicks),
					(AnalyticsEventKind::Scroll, scrolls),
					(AnalyticsEventKind::Error, errors),
				] {
					*self.by_kind.entry(kind).or_default() += count;
				}
				for (country, count) in &rollup.by_country {
					*self.by_country.entry(country.clone()).or_default() +=
						*count as usize;
				}
				for (client_kind, count) in &rollup.by_client_kind {
					*self.by_client_kind.entry(*client_kind).or_default() +=
						*count as usize;
				}
			}
		}
	}

	fn into_summary(self) -> AnalyticsSummary {
		let mut dates = self.dates.into_iter().collect::<Vec<_>>();
		dates.sort();
		AnalyticsSummary {
			total: self.total,
			page_views: self.page_views,
			requests: self.requests,
			visits: self.sessions.len() + self.aggregated_visits,
			dwell: self.dwell,
			scroll: self.scroll,
			by_path: analytics_ext::sort_desc(self.by_path),
			by_kind: analytics_ext::sort_desc(self.by_kind),
			by_client_kind: analytics_ext::sort_desc(self.by_client_kind),
			by_country: analytics_ext::sort_desc(self.by_country),
			broken_links: analytics_ext::sort_desc(self.broken_links),
			probes: self.probes,
			dates,
		}
	}
}

impl core::fmt::Display for AnalyticsSummary {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(
			f,
			"{} events: {} page views, {} requests, {} visits",
			self.total, self.page_views, self.requests, self.visits
		)?;
		match self.span() {
			Some((first, last)) => {
				writeln!(f, "\n{} days, {first} .. {last}", self.dates.len())?
			}
			None => writeln!(f)?,
		}
		let section = |f: &mut core::fmt::Formatter<'_>,
		               title: &str,
		               rows: &[(SmolStr, usize)]|
		 -> core::fmt::Result {
			writeln!(f, "\n{title}:")?;
			for (key, count) in rows.iter().take(20) {
				writeln!(f, "  {count:>6}  {key}")?;
			}
			Ok(())
		};
		// a distribution plus the two figures read off it, never a mean
		let distribution = |f: &mut core::fmt::Formatter<'_>,
		                    title: &str,
		                    buckets: &Buckets,
		                    scale: &BucketScale|
		 -> core::fmt::Result {
			writeln!(f, "\n{title}:")?;
			for (label, count) in buckets.rows(scale) {
				writeln!(f, "  {count:>6}  {label}")?;
			}
			if let (Some(median), Some(p90)) = (
				buckets.percentile(scale, 0.5),
				buckets.percentile(scale, 0.9),
			) {
				writeln!(f, "  median {median}, p90 {p90}")?;
			}
			Ok(())
		};
		writeln!(f, "\nevent kinds:")?;
		for (kind, count) in &self.by_kind {
			writeln!(f, "  {count:>6}  {kind:?}")?;
		}
		writeln!(f, "\nclient kinds:")?;
		for (kind, count) in &self.by_client_kind {
			writeln!(f, "  {count:>6}  {kind:?}")?;
		}
		section(f, "pages", &self.by_path)?;
		section(f, "countries", &self.by_country)?;
		distribution(f, "dwell", &self.dwell, &Buckets::DWELL)?;
		if self.scroll.total() > 0 {
			distribution(f, "scroll depth", &self.scroll, &Buckets::SCROLL)?;
		}
		// the 404s worth acting on, kept clear of the probe flood they would
		// otherwise be buried in.
		if !self.broken_links.is_empty() {
			section(
				f,
				"broken links (404 with a referrer)",
				&self.broken_links,
			)?;
		}
		if self.probes > 0 {
			writeln!(f, "\nunreferred 404s (probes): {}", self.probes)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// An event of `data` at `path` on `date`.
	fn event(
		date: &str,
		path: &str,
		data: AnalyticsEventData,
	) -> AnalyticsEvent {
		let mut event = AnalyticsEvent::new(path, data)
			.with_client_kind(ClientKind::Web)
			.with_session(Some(uuid_ext::now_v7()));
		event.timestamp =
			time_ext::parse_date(date).unwrap().as_millis() as u64;
		event
	}

	/// A request event for `path` with `status`, linked from `referrer`.
	fn request(
		path: &str,
		status: u16,
		referrer: Option<&str>,
	) -> AnalyticsEvent {
		event("2026-08-30", path, AnalyticsEventData::Request {
			status,
			method: "Get".into(),
			user_agent: None,
			referrer: referrer.map(Into::into),
		})
	}

	fn page_view(date: &str, path: &str, duration_ms: u64) -> AnalyticsEvent {
		event(date, path, AnalyticsEventData::PageView {
			duration_ms,
			referrer: None,
			title: None,
			client: default(),
		})
	}

	/// A 404 someone was linked to is a broken link worth naming; a 404 nobody
	/// linked to is a probe, counted but not listed. Without the split a public
	/// site's handful of real broken links is buried under its scanner traffic.
	#[beet_core::test]
	fn splits_broken_links_from_probes() {
		let summary = AnalyticsSummary::from_events(&[
			request("/docs/typo", 404, Some("https://beet.org/docs")),
			request("/docs/typo", 404, Some("https://beet.org/docs")),
			request("/wp-login.php", 404, None),
			request("/.env", 404, None),
			// a served page is neither
			request("/docs", 200, None),
		]);

		summary
			.broken_links
			.xpect_eq(vec![("/docs/typo".into(), 2)]);
		summary.probes.xpect_eq(2);
		summary.requests.xpect_eq(5);
	}

	/// The long history reads out of aggregates, whose figures are the same ones
	/// the raws would have produced. This is what the report says once the raws
	/// have expired.
	#[beet_core::test]
	fn reads_a_history_out_of_aggregates() {
		let events = [
			page_view("2026-08-01", "/", 12_000),
			page_view("2026-08-01", "/docs", 45_000),
			request("/docs/typo", 404, Some("https://beet.org/")),
		];
		let raw = AnalyticsSummary::from_events(&events);
		let aggregated = AnalyticsSummary::from_rollups(
			&AnalyticsRollup::from_events(&events),
		);
		aggregated.page_views.xpect_eq(raw.page_views);
		aggregated.requests.xpect_eq(raw.requests);
		aggregated.total.xpect_eq(raw.total);
		aggregated.by_path.xpect_eq(raw.by_path);
		aggregated.by_client_kind.xpect_eq(raw.by_client_kind);
		aggregated.broken_links.xpect_eq(raw.broken_links);
		aggregated
			.dwell
			.rows(&Buckets::DWELL)
			.xpect_eq(raw.dwell.rows(&Buckets::DWELL));
		// three distinct sessions across the day, from the site-wide row
		aggregated.visits.xpect_eq(3);
	}

	/// A day covered by an aggregate is read from it and NOT from the raws that
	/// are still lying around, so a report over both counts each day once.
	#[beet_core::test]
	fn composes_aggregates_with_the_recent_raws() {
		let old = [page_view("2026-08-01", "/", 12_000)];
		let recent = [page_view("2026-08-29", "/docs", 5_000)];
		let rollups = AnalyticsRollup::from_events(&old);
		// the raws of the aggregated day have not been expired yet
		let summary = AnalyticsSummary::compose(
			&rollups,
			&[old.as_slice(), recent.as_slice()].concat(),
		);
		summary.page_views.xpect_eq(2);
		summary.total.xpect_eq(2);
		summary
			.by_path
			.xpect_eq(vec![("/".into(), 1), ("/docs".into(), 1)]);
		summary.dates.xpect_eq(vec![
			SmolStr::from("2026-08-01"),
			SmolStr::from("2026-08-29"),
		]);
		summary.span().unwrap().1.as_str().xpect_eq("2026-08-29");
	}

	/// Dwell is reported as a distribution with a median and p90 read off it,
	/// and the words `mean dwell` appear nowhere: the mean was ~3.8 hours on the
	/// live site, because an unterminated view averages in at whatever the tab
	/// was left open for.
	#[beet_core::test]
	fn reports_the_dwell_distribution() {
		let report = AnalyticsSummary::from_events(&[
			page_view("2026-08-01", "/", 5_000),
			page_view("2026-08-01", "/", 20_000),
			page_view("2026-08-01", "/", 20 * 60 * 60 * 1000),
		])
		.to_string();
		report
			.as_str()
			.xpect_contains("dwell:")
			.xpect_contains("10-30s")
			.xpect_contains("median 10-30s")
			.xpect_contains("p90 10-30m+")
			.xnot()
			.xpect_contains("mean");
	}
}
