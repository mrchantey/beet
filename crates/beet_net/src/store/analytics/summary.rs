//! The [`AnalyticsSummary`] read model.
use crate::prelude::*;
use beet_core::prelude::*;

/// An aggregate view over a set of [`AnalyticsEvent`]s: what kinds of clients
/// connected, the pages they viewed, and how long for.
///
/// The read model behind `beet analytics` and any dashboard, computed from a
/// plain slice so it needs no storage backend.
#[derive(Debug, Default, Clone)]
pub struct AnalyticsSummary {
	/// Total events across all kinds.
	pub total: usize,
	/// Page-view events (a viewed page with a dwell duration).
	pub page_views: usize,
	/// Request-log events (raw server traffic).
	pub requests: usize,
	/// Distinct sessions seen.
	pub sessions: usize,
	/// Mean page-view dwell in milliseconds (0 when there are no timed views).
	pub mean_dwell_ms: u64,
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
	/// middleware started dropping them at the source, which it now does.
	pub probes: usize,
}

impl AnalyticsSummary {
	/// Aggregates a slice of events into a summary.
	pub fn from_events(events: &[AnalyticsEvent]) -> Self {
		let mut by_path = HashMap::<SmolStr, usize>::default();
		let mut by_kind = HashMap::<AnalyticsEventKind, usize>::default();
		let mut by_client_kind = HashMap::<ClientKind, usize>::default();
		let mut by_country = HashMap::<SmolStr, usize>::default();
		let mut broken_links = HashMap::<SmolStr, usize>::default();
		let mut sessions = HashSet::<Uuid>::default();
		let (mut page_views, mut requests, mut probes) = (0, 0, 0);
		let (mut dwell_sum, mut dwell_count) = (0u64, 0u64);
		for event in events {
			*by_kind.entry(event.event_kind).or_default() += 1;
			match &event.data {
				AnalyticsEventData::PageView { duration_ms, .. } => {
					page_views += 1;
					*by_path.entry(event.path.clone()).or_default() += 1;
					dwell_sum += duration_ms;
					dwell_count += 1;
				}
				AnalyticsEventData::Request {
					status, referrer, ..
				} => {
					requests += 1;
					// a 404 splits by whether anything linked to it: a referred one
					// is a link that should have worked, a cold one is a probe.
					if *status == 404 {
						match referrer {
							Some(_) => {
								*broken_links
									.entry(event.path.clone())
									.or_default() += 1
							}
							None => probes += 1,
						}
					}
				}
				_ => {}
			}
			*by_client_kind.entry(event.client_kind).or_default() += 1;
			if let Some(country) = &event.country {
				*by_country.entry(country.clone()).or_default() += 1;
			}
			if let Some(session) = event.session {
				sessions.insert(session);
			}
		}
		Self {
			total: events.len(),
			page_views,
			requests,
			sessions: sessions.len(),
			mean_dwell_ms: if dwell_count > 0 {
				dwell_sum / dwell_count
			} else {
				0
			},
			by_path: sort_desc(by_path),
			by_kind: sort_desc(by_kind),
			by_client_kind: sort_desc(by_client_kind),
			by_country: sort_desc(by_country),
			broken_links: sort_desc(broken_links),
			probes,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A request event for `path` with `status`, linked from `referrer`.
	fn request(
		path: &str,
		status: u16,
		referrer: Option<&str>,
	) -> AnalyticsEvent {
		AnalyticsEvent::new(path, AnalyticsEventData::Request {
			status,
			method: "Get".into(),
			user_agent: None,
			referrer: referrer.map(Into::into),
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

		summary.broken_links.xpect_eq(vec![("/docs/typo".into(), 2)]);
		summary.probes.xpect_eq(2);
		summary.requests.xpect_eq(5);
	}
}

/// Collect a count map into a vec sorted by descending count.
fn sort_desc<K>(map: HashMap<K, usize>) -> Vec<(K, usize)> {
	let mut counts = map.into_iter().collect::<Vec<_>>();
	counts.sort_by(|left, right| right.1.cmp(&left.1));
	counts
}

impl core::fmt::Display for AnalyticsSummary {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		writeln!(
			f,
			"{} events: {} page views, {} requests, {} sessions",
			self.total, self.page_views, self.requests, self.sessions
		)?;
		writeln!(f, "mean dwell: {}ms", self.mean_dwell_ms)?;
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
		// the 404s worth acting on, kept clear of the probe flood they would
		// otherwise be buried in.
		if !self.broken_links.is_empty() {
			section(f, "broken links (404 with a referrer)", &self.broken_links)?;
		}
		if self.probes > 0 {
			writeln!(f, "\nunreferred 404s (probes): {}", self.probes)?;
		}
		Ok(())
	}
}
