//! The nightly analytics job: each complete day rolled up, then each cold day
//! archived.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// One run of the nightly job over an [`AnalyticsStore`].
///
/// Each complete UTC day after the last one rolled up is scanned (one key
/// range of `analytics_event`), reduced to [`AnalyticsRollup`] rows, and each
/// row written and read back. The rows are keyed by day and scope, so a rerun
/// rewrites the same rows: idempotent, and `full` rebuilds every day from its
/// rows and its archive both, for a new aggregate schema. Then every day older than `keep_days` is moved to the
/// archive bucket as one SQLite file, `analytics/events/<date>.db`
/// ([`TableArchive`]), and its rows retired from the table, so the hot table
/// holds a bounded window while the aggregates and the archives hold
/// everything.
pub struct AnalyticsRollupRun {
	store: AnalyticsStore,
	archive: Option<BlobStore>,
	keep_days: u32,
	full: bool,
}

impl AnalyticsRollupRun {
	/// The raw days a run keeps in the table by default before archiving.
	pub const KEEP_DAYS: u32 = 30;

	/// A run over `store`, archiving into `archive` when given.
	pub fn new(store: AnalyticsStore, archive: Option<BlobStore>) -> Self {
		Self {
			store,
			archive,
			keep_days: Self::KEEP_DAYS,
			full: false,
		}
	}

	/// Rebuild every day, archived ones included, not only the new ones.
	pub fn with_full(mut self, full: bool) -> Self {
		self.full = full;
		self
	}

	/// Keep `keep_days` complete days of raw events before archiving.
	pub fn with_keep_days(mut self, keep_days: u32) -> Self {
		self.keep_days = keep_days;
		self
	}

	/// Where a day's archive lives in the archive bucket.
	pub fn archive_path(date: Date) -> RelPath {
		RelPath::new(format!("analytics/events/{date}.db"))
	}

	/// Roll up, then archive, reporting the verified work.
	pub async fn call(&self) -> Result<AnalyticsRollupReport> {
		let today = Date::today();
		let mut report = AnalyticsRollupReport {
			full: self.full,
			..default()
		};
		let first = self.store.first_day().await?;
		let archived = match self.full {
			true => self.archived_days().await?,
			false => Vec::new(),
		};
		let Some(start) =
			first.into_iter().chain(archived.iter().copied()).min()
		else {
			return report.xok();
		};
		let start = match (self.full, self.store.last_rolled_day().await?) {
			(false, Some(last)) => last.add_days(1).max(start),
			_ => start,
		};
		let mut date = start;
		while date < today {
			let events = self.day(date, archived.contains(&date)).await?;
			report.scanned += events.len() as u32;
			report.rollups += self.roll_up(&events).await?;
			if !events.is_empty() {
				report.dates.push(date.to_string().into());
			}
			date = date.add_days(1);
		}
		if let (Some(archive), Some(first)) = (&self.archive, first) {
			cfg_if! {
				if #[cfg(feature = "sqlite")] {
					self.archive(archive, first, today, &mut report).await?;
				} else {
					let _ = (archive, first);
					bevybail!(
						"archiving analytics needs the `sqlite` feature: each cold \
						 day is archived as a SQLite file"
					);
				}
			}
		}
		report.xok()
	}

	/// Write one day's aggregates and read each back, answering how many. The
	/// site-wide row goes last, alone: its presence is what marks the day
	/// complete ([`AnalyticsStore::last_rolled_day`]), so a run that dies
	/// partway leaves a day the next run redoes.
	async fn roll_up(&self, events: &[AnalyticsEvent]) -> Result<u32> {
		let table = self.store.rollups();
		let write = async |row: AnalyticsRollup| -> Result {
			table.put(row.clone()).await?;
			if table.get(row.key()).await?.as_ref() != Some(&row) {
				bevybail!(
					"analytics rollup row {} for {} was written to {} but failed read-back verification",
					row.id,
					row.date,
					table.describe()
				);
			}
			Ok(())
		};
		let (site, paths): (Vec<_>, Vec<_>) =
			AnalyticsRollup::from_events(events)
				.into_iter()
				.partition(|row| row.scope == AnalyticsScope::Site);
		let written = site.len() + paths.len();
		async_ext::try_join_all_bounded(
			BlobStore::GET_ALL_CONCURRENCY,
			paths.into_iter().map(&write),
		)
		.await?;
		for row in site {
			write(row).await?;
		}
		(written as u32).xok()
	}

	/// The events of `date`: the table's, and its archive's when it has one,
	/// so a rebuild never sees a day some of whose rows were retired.
	async fn day(
		&self,
		date: Date,
		archived: bool,
	) -> Result<Vec<AnalyticsEvent>> {
		let events = self.store.day(date).await?;
		let Some(archive) = self.archive.as_ref().filter(|_| archived) else {
			return events.xok();
		};
		cfg_if! {
			if #[cfg(feature = "sqlite")] {
				let cold =
					TableArchive::open(archive, &Self::archive_path(date))
						.await?;
				let mut all = events;
				all.extend(
					Table::<AnalyticsEvent>::new(cold.clone()).get_all().await?,
				);
				cold.store_remove().await?;
				AnalyticsEvent::dedupe(all).xok()
			} else {
				// the table alone would rebuild the day short of its history
				bevybail!(
					"rebuilding {date} needs the `sqlite` feature: its events \
					 are archived as a SQLite file in {}",
					archive.describe()
				);
			}
		}
	}

	/// Every day the archive bucket holds a file for.
	async fn archived_days(&self) -> Result<Vec<Date>> {
		let Some(archive) = &self.archive else {
			return Vec::new().xok();
		};
		archive
			.list()
			.await?
			.into_iter()
			.filter_map(|path| {
				path.as_str()
					.strip_prefix("analytics/events/")?
					.strip_suffix(".db")
					.and_then(|date| Date::parse(date).ok())
			})
			.collect::<Vec<_>>()
			.xok()
	}

	/// Archive each day from `first` past the window whose aggregate is
	/// already written.
	#[cfg(feature = "sqlite")]
	async fn archive(
		&self,
		archive: &BlobStore,
		first: Date,
		today: Date,
		report: &mut AnalyticsRollupReport,
	) -> Result {
		let cutoff = today.add_days(-(self.keep_days as i64));
		let rolled = self.store.last_rolled_day().await?;
		let mut date = first;
		while date < cutoff && rolled.is_some_and(|rolled| date <= rolled) {
			let path = Self::archive_path(date);
			let moved = TableArchive::new(
				&self.store.events(),
				AnalyticsEvent::day_scan(date),
			)
			.run(archive, &path)
			.await?;
			if moved.rows > 0 {
				report.archived.push(path.to_string().into());
				report.retired += moved.rows as u32;
			}
			date = date.add_days(1);
		}
		Ok(())
	}
}

/// The verified work of one [`AnalyticsRollupRun`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsRollupReport {
	/// Whether the run rebuilt every day, archived ones included.
	pub full: bool,
	/// Raw events reduced, heartbeats deduplicated.
	pub scanned: u32,
	/// The days rolled up, ascending.
	pub dates: Vec<SmolStr>,
	/// Aggregate rows written and verified.
	pub rollups: u32,
	/// Archive files written and verified.
	pub archived: Vec<SmolStr>,
	/// Raw events moved out of the table into those archives.
	pub retired: u32,
}

impl core::fmt::Display for AnalyticsRollupReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		writeln!(
			f,
			"analytics {}: {} days from {} raw events",
			if self.full { "backfill" } else { "rollup" },
			self.dates.len(),
			self.scanned
		)?;
		if let (Some(first), Some(last)) =
			(self.dates.first(), self.dates.last())
		{
			writeln!(f, "  days:       {first} .. {last}")?;
		}
		writeln!(f, "  aggregates: {} rows", self.rollups)?;
		write!(
			f,
			"  archived:   {} days, {} events",
			self.archived.len(),
			self.retired
		)
	}
}

/// Rolls up each complete day of analytics and archives the cold ones.
///
/// The route names its two stores as fields of the job itself, one per role,
/// so a missing one is an error by field name: the table store the events
/// and aggregates live in, and the bucket cold days are archived to. A run is
/// idempotent, see [`AnalyticsRollupRun`].
#[action]
#[derive(Default, Component, Reflect, MapEntities)]
#[reflect(Component, Default, MapEntities)]
pub async fn AnalyticsRollupJob(
	/// The table store holding the `analytics_event` and `analytics_rollup`
	/// tables, the one the site's `AnalyticsConfig` records to.
	#[field(required)]
	#[entities]
	analytics: Entity,
	/// The bucket each cold day is archived to as a SQLite file.
	#[field(required)]
	#[entities]
	archive: Entity,
	/// Rebuilds every day, archived ones included, not only new days.
	#[param]
	full: bool,
	/// Complete days of raw events kept in the table before archiving.
	#[param]
	keep_days: Option<u32>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let world = cx.caller.world().clone();
	let report = AnalyticsRollupRun::new(
		AnalyticsStore::new(
			StoreRef::resolve::<TableStore>(&world, analytics).await?,
		),
		Some(StoreRef::resolve::<BlobStore>(&world, archive).await?),
	)
	.with_full(full)
	.with_keep_days(keep_days.unwrap_or(AnalyticsRollupRun::KEEP_DAYS))
	.call()
	.await?;
	info!("{report}");
	Response::ok_text(report.to_string()).xok()
}

#[cfg(test)]
mod test {
	use super::*;
	use beet_action::prelude::*;

	const DAY_MS: u64 = 86_400_000;

	fn page_view(days: u64, path: &str, session: u128) -> AnalyticsEvent {
		AnalyticsEvent::new(path, AnalyticsEventData::PageView {
			duration_ms: 12_000,
			referrer: None,
			title: None,
			client: default(),
		})
		.with_session(Some(Uuid::from_u128(session)))
		.with_timestamp(Timestamp::now() - Duration::from_millis(days * DAY_MS))
	}

	async fn recorded(events: &[AnalyticsEvent]) -> AnalyticsStore {
		let store = AnalyticsStore::new(TableStore::temp());
		for event in events {
			store.record(event.clone()).await.unwrap();
		}
		store
	}

	/// Complete days roll up, today does not, and a rerun rewrites the same
	/// rows.
	#[beet_core::test]
	async fn rolls_up_complete_days() {
		let store = recorded(&[
			page_view(2, "/a", 1),
			page_view(2, "/b", 1),
			page_view(1, "/a", 2),
			page_view(0, "/today", 3),
		])
		.await;
		let report = AnalyticsRollupRun::new(store.clone(), None)
			.call()
			.await
			.unwrap();
		report.dates.len().xpect_eq(2);
		report.scanned.xpect_eq(3);
		let yesterday = Date::today().add_days(-1);
		let rows = store.rollups().get_all().await.unwrap();
		let site = rows
			.iter()
			.find(|row| {
				row.date == yesterday.to_string()
					&& row.scope == AnalyticsScope::Site
			})
			.unwrap();
		site.views.xpect_eq(1);
		rows.iter()
			.any(|row| row.date == Date::today().to_string())
			.xpect_false();
		// the next run has nothing new, a full one rebuilds the same rows
		AnalyticsRollupRun::new(store.clone(), None)
			.call()
			.await
			.unwrap()
			.dates
			.xpect_empty();
		AnalyticsRollupRun::new(store.clone(), None)
			.with_full(true)
			.call()
			.await
			.unwrap()
			.dates
			.len()
			.xpect_eq(2);
		store
			.rollups()
			.get_all()
			.await
			.unwrap()
			.len()
			.xpect_eq(rows.len());
	}

	/// A day past the window is archived into its own file and leaves the
	/// table, its aggregate kept; the summary still counts it.
	#[cfg(feature = "sqlite")]
	#[beet_core::test]
	async fn archives_cold_days() {
		let store =
			recorded(&[page_view(40, "/cold", 1), page_view(1, "/warm", 2)])
				.await;
		let archive = BlobStore::temp();
		let report =
			AnalyticsRollupRun::new(store.clone(), Some(archive.clone()))
				.call()
				.await
				.unwrap();
		let cold = Date::today().add_days(-40);
		report.archived.xpect_eq(vec![SmolStr::from(
			AnalyticsRollupRun::archive_path(cold).to_string(),
		)]);
		report.retired.xpect_eq(1);
		store.day(cold).await.unwrap().xpect_empty();
		store.events().list().await.unwrap().len().xpect_eq(1);
		Table::<AnalyticsEvent>::new(
			TableArchive::open(
				&archive,
				&AnalyticsRollupRun::archive_path(cold),
			)
			.await
			.unwrap(),
		)
		.get_all()
		.await
		.unwrap()[0]
			.path
			.as_str()
			.xpect_eq("/cold");
		store.summary(false).await.unwrap().page_views.xpect_eq(2);
		// a rerun moves nothing more
		AnalyticsRollupRun::new(store, Some(archive))
			.call()
			.await
			.unwrap()
			.retired
			.xpect_eq(0);
	}

	/// The job runs in a Worker under `wrangler dev`, its events in Durable
	/// Objects and its archive built in the Worker's own SQLite (`sqlite-wasm-rs`
	/// in workerd's memory) and written to R2: a document mounting
	/// `<DurableObjectStore bx:ref="tables" binding="TABLES"><ServeTables
	/// prefix="tables"/></DurableObjectStore>`, `<R2WorkersStore
	/// bx:ref="archive" binding="ARCHIVE"/>` and `<Route path="rollup"
	/// {AnalyticsRollupJob{analytics: $tables, archive: $archive}}/>`.
	#[cfg(all(feature = "ureq", not(target_arch = "wasm32")))]
	#[beet_core::test(timeout_ms = 600_000)]
	#[ignore = "needs a Worker under wrangler dev on 127.0.0.1:8787"]
	async fn archives_in_a_worker() {
		let store = AnalyticsStore::new(TableStore::new(
			TableWireClient::connect(
				TableWireTransport::http("http://127.0.0.1:8787/tables")
					.unwrap(),
			)
			.await
			.unwrap(),
		));
		store.record(page_view(3, "/cold", 1)).await.unwrap();
		Request::post("http://127.0.0.1:8787/rollup?keep_days=0")
			.send()
			.await
			.unwrap()
			.text()
			.await
			.unwrap()
			.xpect_contains("archived:   1 days, 1 events");
		store
			.day(Date::today().add_days(-3))
			.await
			.unwrap()
			.xpect_empty();
	}

	/// A full run rebuilds an archived day from its archive, so a straggler
	/// left in the table never overwrites the day's aggregate with itself.
	#[cfg(feature = "sqlite")]
	#[beet_core::test]
	async fn full_rebuilds_from_the_archive() {
		let store =
			recorded(&[page_view(40, "/a", 1), page_view(40, "/b", 2)]).await;
		let archive = BlobStore::temp();
		AnalyticsRollupRun::new(store.clone(), Some(archive.clone()))
			.call()
			.await
			.unwrap()
			.retired
			.xpect_eq(2);
		store.record(page_view(40, "/late", 3)).await.unwrap();
		AnalyticsRollupRun::new(store.clone(), Some(archive))
			.with_full(true)
			.call()
			.await
			.unwrap()
			.scanned
			.xpect_eq(3);
		store.summary(false).await.unwrap().page_views.xpect_eq(3);
	}

	/// A day whose run died before its site-wide row is not complete: the
	/// summary reads its raw events, and the next run redoes it.
	#[beet_core::test]
	async fn redoes_a_day_cut_short() {
		let events = [
			page_view(2, "/a", 1),
			page_view(1, "/a", 2),
			page_view(1, "/b", 3),
		];
		let store = recorded(&events).await;
		for row in
			AnalyticsRollup::from_events(&events)
				.into_iter()
				.filter(|row| {
					row.date == events[0].date()
						|| row.scope == AnalyticsScope::Path("/a".into())
				}) {
			store.rollups().put(row).await.unwrap();
		}
		store.summary(false).await.unwrap().page_views.xpect_eq(3);
		AnalyticsRollupRun::new(store.clone(), None)
			.call()
			.await
			.unwrap()
			.dates
			.xpect_eq(vec![events[1].date()]);
		store.summary(false).await.unwrap().page_views.xpect_eq(3);
		store.visits().await.unwrap().xpect_eq(3);
	}

	/// The job resolves its two stores by field and reports.
	#[beet_core::test]
	async fn job_runs_from_its_fields() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let analytics = world.spawn(InMemoryStore::new()).flush();
		let archive = world.spawn(InMemoryStore::new()).flush();
		let store = AnalyticsStore::new(
			world.entity(analytics).get::<TableStore>().unwrap().clone(),
		);
		store.record(page_view(1, "/a", 1)).await.unwrap();
		let job = world
			.spawn(AnalyticsRollupJob {
				analytics: Some(analytics),
				archive: Some(archive),
			})
			.flush();
		world
			.entity_mut(job)
			.call::<Request, Response>(Request::new(HttpMethod::Post, "rollup"))
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_contains("1 days from 1 raw events");
	}
}
