//! Analytics compaction: archives and rollups are verified before shards delete.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Names the blob store a rollup job writes [`AnalyticsRollup`] rows to.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RollupStoreConsumers, allow_self_referential)]
pub struct RollupStoreRef(#[entities] pub Entity);

/// Tracks every job bound to an aggregate store.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = RollupStoreRef)]
pub struct RollupStoreConsumers(Vec<Entity>);

/// Names the blob store a rollup job writes daily raw archives to.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(
	relationship_target = ArchiveStoreConsumers,
	allow_self_referential
)]
pub struct ArchiveStoreRef(#[entities] pub Entity);

/// Tracks every job bound to an archive store.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ArchiveStoreRef)]
pub struct ArchiveStoreConsumers(Vec<Entity>);

/// Runs analytics compaction over raw, rollup, and archive blob stores.
///
/// Complete-day segments are merged with that day's existing archive and
/// deduplicated by event ID. Every archive and JSON-over-blob rollup row is read
/// back and compared before any source segment is deleted, making retries safe
/// after a failure at every stage.
pub struct AnalyticsRollupRun {
	raw: BlobStore,
	rollups: Table<AnalyticsRollup>,
	archive: BlobStore,
	full: bool,
}

impl AnalyticsRollupRun {
	/// Creates a run over raw, rollup, and archive blob stores.
	///
	/// The three may be one store: each keyspace is a disjoint prefix, so a
	/// deployment collapses them by pointing two refs at one declaration.
	pub fn new(raw: BlobStore, rollups: BlobStore, archive: BlobStore) -> Self {
		Self {
			raw,
			rollups: AnalyticsRollup::table(rollups),
			archive,
			full: false,
		}
	}

	/// Configures whether every archived complete day is rebuilt.
	pub fn with_full(mut self, full: bool) -> Self {
		self.full = full;
		self
	}

	/// Compacts all eligible days and reports the verified work.
	pub async fn call(&self) -> Result<AnalyticsRollupReport> {
		// the eligible dates are decided from paths alone, so the current day's
		// segments (the bulk of the keyspace) are never fetched to be discarded
		let dates = self.dates().await?;
		let mut segments =
			HashMap::<SmolStr, Vec<(SmolPath, Vec<AnalyticsEvent>)>>::default();
		for (path, events) in
			AnalyticsSegment::read_dates(&self.raw, |date| dates.contains(date))
				.await?
		{
			let date = AnalyticsSegment::date(&path).ok_or_else(|| {
				bevyhow!("invalid analytics segment path `{path}`")
			})?;
			segments.entry(date).or_default().push((path, events));
		}

		let mut scanned = 0;
		let mut days = Vec::new();
		for date in &dates {
			let mut events = AnalyticsArchive::read(&self.archive, date)
				.await?
				.unwrap_or_default();
			let mut paths = Vec::new();
			if let Some(shards) = segments.remove(date) {
				for (path, shard_events) in shards {
					paths.push(path);
					events.extend(shard_events);
				}
			}
			scanned += events.len() as u32;
			days.push(AnalyticsDay {
				date: date.clone(),
				events: AnalyticsStore::dedupe(events),
				segments: paths,
			});
		}

		// Nothing below may delete a shard until every archive has exact readback.
		let mut archived = Vec::new();
		for day in &days {
			archived.push(
				AnalyticsArchive::write(&self.archive, &day.date, &day.events)
					.await?
					.to_string()
					.into(),
			);
		}

		// Force the blanket JSON-over-Blob TableProvider and verify each value.
		let writes = days
			.iter()
			.flat_map(|day| AnalyticsRollup::from_events(&day.events))
			.map(async |row| {
				let id = row.id;
				self.rollups.push(row.clone()).await?;
				let actual = self.rollups.get(id).await?;
				if actual != row {
					bevybail!(
						"analytics rollup row {id} for {} was written to {} but failed read-back verification",
						row.date,
						self.rollups.describe()
					);
				}
				Ok(())
			});
		let rollups = Self::drive(writes).await?;

		// Archive and every aggregate are durable; source shards may now disappear.
		let deletes = days
			.iter()
			.flat_map(|day| day.segments.iter().cloned())
			.map(async |path| {
				self.raw.remove(&path).await?;
				if self.raw.exists(&path).await? {
					bevybail!(
						"analytics segment `{path}` still exists in {} after deletion",
						self.raw.describe()
					);
				}
				Ok(())
			});
		let deleted = Self::drive(deletes).await?;

		AnalyticsRollupReport {
			full: self.full,
			scanned,
			dates,
			archived,
			rollups,
			deleted,
		}
		.xok()
	}

	/// Returns complete days requiring compaction or aggregate recovery.
	///
	/// A day is complete once UTC has moved past it, so the current day is
	/// always left alone: its segments are still being appended to and an
	/// archive written now would be replaced by the next run anyway.
	async fn dates(&self) -> Result<Vec<SmolStr>> {
		let today = Timestamp::now().format_date();
		let mut dates = AnalyticsSegment::dates(&self.raw)
			.await?
			.into_iter()
			.filter(|date| date.as_str() < today.as_str())
			.collect::<HashSet<_>>();
		for date in AnalyticsArchive::dates(&self.archive)
			.await?
			.into_iter()
			.filter(|date| date.as_str() < today.as_str())
		{
			if self.full || !self.covered(&date).await? {
				dates.insert(date);
			}
		}
		let mut dates = dates.into_iter().collect::<Vec<_>>();
		dates.sort();
		dates.xok()
	}

	/// Returns whether a date has its archive and site-wide aggregate.
	async fn covered(&self, date: &str) -> Result<bool> {
		Ok(self
			.archive
			.exists(&AnalyticsArchive::object_path(date))
			.await? && self
			.rollups
			.exists(AnalyticsRollup::row_id(date, &AnalyticsScope::Site))
			.await?)
	}

	/// Drives bounded store operations and returns their completed count.
	async fn drive(
		writes: impl Iterator<Item = impl Future<Output = Result>>,
	) -> Result<u32> {
		let results =
			async_ext::join_all_bounded(BlobStore::GET_ALL_CONCURRENCY, writes)
				.await;
		let total = results.len() as u32;
		results.into_iter().collect::<Result<Vec<_>>>()?;
		total.xok()
	}
}

/// Holds one complete day's merged compaction input.
struct AnalyticsDay {
	date: SmolStr,
	events: Vec<AnalyticsEvent>,
	segments: Vec<SmolPath>,
}

/// Reports the verified work completed by one [`AnalyticsRollupRun`].
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsRollupReport {
	/// Whether the run rebuilt every complete archived day.
	pub full: bool,
	/// Raw records merged before event-ID deduplication.
	pub scanned: u32,
	/// Complete UTC dates compacted in ascending order.
	pub dates: Vec<SmolStr>,
	/// Daily archive objects written and verified.
	pub archived: Vec<SmolStr>,
	/// Aggregate rows written and verified.
	pub rollups: u32,
	/// Source segment objects deleted after verification.
	pub deleted: u32,
}

impl core::fmt::Display for AnalyticsRollupReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		writeln!(
			f,
			"analytics {}: {} days from {} raw records",
			if self.full { "backfill" } else { "rollup" },
			self.dates.len(),
			self.scanned
		)?;
		if let (Some(first), Some(last)) =
			(self.dates.first(), self.dates.last())
		{
			writeln!(f, "  days:       {first} .. {last}")?;
		}
		writeln!(f, "  archived:   {} objects", self.archived.len())?;
		writeln!(f, "  aggregates: {} rows", self.rollups)?;
		write!(f, "  deleted:    {} segments", self.deleted)
	}
}

/// Defines request parameters for [`AnalyticsRollupJob`].
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsRollupParams {
	/// Rebuilds every complete archived day, not only new or uncovered days.
	full: Option<bool>,
}

/// Compacts raw analytics segments into daily archives and aggregates.
///
/// The route names its raw, rollup, and archive stores by relation. A run is
/// idempotent: archive paths and aggregate IDs are deterministic, while event-ID
/// deduplication makes a retry safe if a prior run archived but did not delete.
#[action]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsRollupParams>())]
pub async fn AnalyticsRollupJob(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let world = caller.world().clone();
	let full = cx.input.request_parts().has_param("full");
	let declared = async |name: &str, target: Result<Entity>| match target {
		Ok(target) => Ok(target),
		Err(_) => bevybail!(
			"the analytics rollup job names the stores it works on: add \
			 `{name}($declaration)` beside it, pointing at the block that \
			 declares the store"
		),
	};
	let raw = declared(
		"StoreRef",
		caller
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await,
	)
	.await?;
	let rollups = declared(
		"RollupStoreRef",
		caller
			.get::<RollupStoreRef, _>(|store_ref| store_ref.0)
			.await,
	)
	.await?;
	let archive = declared(
		"ArchiveStoreRef",
		caller
			.get::<ArchiveStoreRef, _>(|store_ref| store_ref.0)
			.await,
	)
	.await?;

	let report = AnalyticsRollupRun::new(
		StoreRef::resolve::<BlobStore>(&world, raw).await?,
		StoreRef::resolve::<BlobStore>(&world, rollups).await?,
		StoreRef::resolve::<BlobStore>(&world, archive).await?,
	)
	.with_full(full)
	.call()
	.await?;
	info!("{report}");
	Response::ok_text(report.to_string()).xok()
}

#[cfg(test)]
mod test {
	use crate::exports::bytes::Bytes;
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	const DAY_MS: u64 = 86_400_000;

	fn run() -> (
		AnalyticsRollupRun,
		BlobStore,
		Table<AnalyticsRollup>,
		BlobStore,
	) {
		let raw = BlobStore::temp();
		let rollup_store = BlobStore::temp();
		let archive = BlobStore::temp();
		(
			AnalyticsRollupRun::new(
				raw.clone(),
				rollup_store.clone(),
				archive.clone(),
			),
			raw,
			AnalyticsRollup::table(rollup_store),
			archive,
		)
	}

	fn page_view(days: u64, path: &str, session: u128) -> AnalyticsEvent {
		let mut event =
			AnalyticsEvent::new(path, AnalyticsEventData::PageView {
				duration_ms: 12_000,
				referrer: None,
				title: None,
				client: default(),
			})
			.with_session(Some(Uuid::from_u128(session)));
		event.timestamp = analytics_ext::now_ms() - days * DAY_MS;
		event
	}

	async fn write_segment(
		store: &BlobStore,
		events: &[AnalyticsEvent],
		sequence: u64,
	) -> SmolPath {
		let path = AnalyticsSegment::object_path(
			&events[0].date(),
			Uuid::from_u128(sequence as u128 + 1),
			events[0].timestamp,
			sequence,
		);
		AnalyticsSegment::write(store, path.clone(), events)
			.await
			.unwrap();
		path
	}

	#[beet_core::test]
	async fn archives_rolls_up_then_deletes_segments() {
		let (run, raw, rollups, archive) = run();
		let sources = [
			page_view(3, "/", 1),
			page_view(3, "/docs", 1),
			page_view(2, "/", 2),
		];
		let older_path = write_segment(&raw, &sources[..2], 1).await;
		let newer_path = write_segment(&raw, &sources[2..], 2).await;
		let report = run.call().await.unwrap();
		report.scanned.xpect_eq(3);
		report.dates.len().xpect_eq(2);
		report.archived.len().xpect_eq(2);
		report.rollups.xpect_eq(5);
		report.deleted.xpect_eq(2);
		raw.exists(&older_path).await.unwrap().xpect_false();
		raw.exists(&newer_path).await.unwrap().xpect_false();

		let date = sources[0].date();
		AnalyticsArchive::read(&archive, &date)
			.await
			.unwrap()
			.unwrap()
			.len()
			.xpect_eq(2);
		let site = rollups
			.get(AnalyticsRollup::row_id(&date, &AnalyticsScope::Site))
			.await
			.unwrap();
		site.views.xpect_eq(2);
		site.visits.xpect_eq(1);
	}

	/// The current day is still being appended to, so it is neither compacted
	/// nor read: the unreadable object proves the second half, since decoding it
	/// would fail the run rather than leave it alone.
	#[beet_core::test]
	async fn leaves_today_segments_alone() {
		let (run, raw, ..) = run();
		let today = page_view(0, "/", 1);
		let path = write_segment(&raw, &[today.clone()], 1).await;
		let corrupt = AnalyticsSegment::object_path(
			&today.date(),
			Uuid::from_u128(9),
			today.timestamp,
			9,
		);
		raw.insert(&corrupt, Bytes::from_static(b"not gzip"))
			.await
			.unwrap();
		let report = run.call().await.unwrap();
		report.dates.is_empty().xpect_true();
		report.deleted.xpect_eq(0);
		raw.exists(&path).await.unwrap().xpect_true();
		raw.exists(&corrupt).await.unwrap().xpect_true();
	}

	#[beet_core::test]
	async fn merges_archive_and_late_segments_with_deduplication() {
		let (run, raw, rollups, archive) = run();
		let old = page_view(2, "/docs", 1);
		AnalyticsArchive::write(&archive, &old.date(), &[old.clone()])
			.await
			.unwrap();
		for row in AnalyticsRollup::from_events(&[old.clone()]) {
			rollups.push(row).await.unwrap();
		}
		let mut newest = old.clone();
		newest.timestamp += 1;
		newest.data = AnalyticsEventData::PageView {
			duration_ms: 24_000,
			referrer: None,
			title: None,
			client: default(),
		};
		let path = write_segment(&raw, &[newest.clone()], 1).await;

		let report = run.call().await.unwrap();
		report.scanned.xpect_eq(2);
		report.dates.xpect_eq(vec![old.date()]);
		report.deleted.xpect_eq(1);
		raw.exists(&path).await.unwrap().xpect_false();
		let archived = AnalyticsArchive::read(&archive, &old.date())
			.await
			.unwrap()
			.unwrap();
		archived.len().xpect_eq(1);
		archived[0].timestamp.xpect_eq(newest.timestamp);
	}

	#[beet_core::test]
	async fn recovers_uncovered_archives_and_supports_full_rebuilds() {
		let (run, _raw, rollups, archive) = run();
		let event = page_view(2, "/", 1);
		AnalyticsArchive::write(&archive, &event.date(), &[event.clone()])
			.await
			.unwrap();
		run.call().await.unwrap().dates.len().xpect_eq(1);
		run.call().await.unwrap().dates.is_empty().xpect_true();
		rollups
			.remove(AnalyticsRollup::row_id(
				&event.date(),
				&AnalyticsScope::Site,
			))
			.await
			.unwrap();
		run.call().await.unwrap().dates.len().xpect_eq(1);
		run.with_full(true)
			.call()
			.await
			.unwrap()
			.dates
			.len()
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn dispatches_over_blob_store_relations() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let stores = [(); 3].map(|_| world.spawn(InMemoryStore::new()).flush());
		let [raw, rollups, archive] = stores;
		let event = page_view(2, "/docs", 1);
		let raw_store = world.entity(raw).get::<BlobStore>().unwrap().clone();
		write_segment(&raw_store, &[event.clone()], 1).await;
		let job = world
			.spawn((
				AnalyticsRollupJob::default(),
				StoreRef(raw),
				RollupStoreRef(rollups),
				ArchiveStoreRef(archive),
			))
			.flush();

		let report = world
			.entity_mut(job)
			.call::<Request, Response>(Request::new(
				HttpMethod::Post,
				"rollup?full=true",
			))
			.await
			.unwrap()
			.unwrap_str()
			.await;
		report
			.as_str()
			.xpect_contains("analytics backfill: 1 days")
			.xpect_contains("deleted:    1 segments");
		let rollup_store =
			world.entity(rollups).get::<BlobStore>().unwrap().clone();
		AnalyticsRollup::table(rollup_store)
			.get(AnalyticsRollup::row_id(
				&event.date(),
				&AnalyticsScope::Site,
			))
			.await
			.unwrap()
			.views
			.xpect_eq(1);
	}

	#[beet_core::test]
	async fn an_unpointed_job_fails_loudly() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let store = world.spawn(InMemoryStore::new()).flush();
		let unpointed = world.spawn(AnalyticsRollupJob::default()).flush();
		let no_rollups = world
			.spawn((AnalyticsRollupJob::default(), StoreRef(store)))
			.flush();
		let no_archive = world
			.spawn((
				AnalyticsRollupJob::default(),
				StoreRef(store),
				RollupStoreRef(store),
			))
			.flush();
		for (job, missing) in [
			(unpointed, "StoreRef"),
			(no_rollups, "RollupStoreRef"),
			(no_archive, "ArchiveStoreRef"),
		] {
			world
				.entity_mut(job)
				.call::<Request, Response>(Request::new(
					HttpMethod::Post,
					"rollup",
				))
				.await
				.unwrap_err()
				.to_string()
				.xpect_contains(missing);
		}
	}
}
