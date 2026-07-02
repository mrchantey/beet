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
	pub by_kind: Vec<(AnalyticsKind, usize)>,
	/// Event counts per client kind, most-common first.
	pub by_client_kind: Vec<(ClientKind, usize)>,
	/// Event counts per country code, most-common first.
	pub by_country: Vec<(SmolStr, usize)>,
}

impl AnalyticsSummary {
	/// Aggregates a slice of events into a summary.
	pub fn from_events(events: &[AnalyticsEvent]) -> Self {
		let mut by_path = HashMap::<SmolStr, usize>::default();
		let mut by_kind = HashMap::<AnalyticsKind, usize>::default();
		let mut by_client_kind = HashMap::<ClientKind, usize>::default();
		let mut by_country = HashMap::<SmolStr, usize>::default();
		let mut sessions = HashSet::<Uuid>::default();
		let (mut page_views, mut requests) = (0, 0);
		let (mut dwell_sum, mut dwell_count) = (0u64, 0u64);
		for event in events {
			*by_kind.entry(event.kind).or_default() += 1;
			match event.kind {
				AnalyticsKind::PageView => {
					page_views += 1;
					*by_path.entry(event.path.clone()).or_default() += 1;
					if let Some(dwell) = event.duration_ms {
						dwell_sum += dwell;
						dwell_count += 1;
					}
				}
				AnalyticsKind::Request => requests += 1,
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
		}
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
		Ok(())
	}
}
