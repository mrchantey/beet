//! The nightly job: archive a day of raw events cold, roll it up, then let it
//! expire — in that order and no other.
use crate::prelude::*;
use beet_core::prelude::*;
use serde::Deserialize;
use serde::Serialize;

/// Names the table a rollup job writes its [`AnalyticsRollup`] rows to, the
/// [`StoreRef`] twin for the aggregate half.
///
/// A second table rather than a corner of the events one: mixing row shapes
/// would make every event scan warn-skip the aggregates, and the expiry the
/// events table declares must never apply to rows that are meant to outlive
/// them.
///
/// ```html
/// <DynamoTableBlock bx:ref="rollup" label="analytics-rollup"/>
/// <Route path="rollup" {(AnalyticsRollupJob, RollupStoreRef($rollup))}/>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = RollupStoreConsumers, allow_self_referential)]
pub struct RollupStoreRef(#[entities] pub Entity);

/// Every job bound to an aggregate store: the target half of
/// [`RollupStoreRef`].
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = RollupStoreRef)]
pub struct RollupStoreConsumers(Vec<Entity>);

/// Names the store a rollup job archives raw events into, ie the bucket for
/// durable data born at runtime rather than published by a deploy.
///
/// Its own relation because it is its own resource with its own grant: the
/// bucket a deploy syncs content into is pruned by that sync and read-only to
/// the running process, which is the opposite of what an archive needs.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(
	relationship_target = ArchiveStoreConsumers,
	allow_self_referential
)]
pub struct ArchiveStoreRef(#[entities] pub Entity);

/// Every job bound to an archive store: the target half of
/// [`ArchiveStoreRef`].
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = ArchiveStoreRef)]
pub struct ArchiveStoreConsumers(Vec<Entity>);

/// One run of the analytics retention pipeline over the three stores it
/// touches.
///
/// The order is the whole design and it is enforced here rather than trusted to
/// a caller: **archive every covered day cold, roll every covered day up, and
/// only then stamp an expiry on the raws those days hold.** Each step confirms
/// its own write by reading it back, so a store that accepted something it did
/// not keep stops the run before anything becomes expirable.
///
/// The raw history is a few megabytes compressed and the aggregates are
/// forever, so expiring the table is a cost decision, never information loss.
pub struct AnalyticsRollupRun {
	events: Table<AnalyticsEvent>,
	rollups: Table<AnalyticsRollup>,
	archive: BlobStore,
	retention: AnalyticsRetention,
	full: bool,
}

impl AnalyticsRollupRun {
	/// A run over the events table, the aggregate table and the archive store,
	/// covering the complete days not yet archived AND rolled up.
	pub fn new(
		events: Table<AnalyticsEvent>,
		rollups: Table<AnalyticsRollup>,
		archive: BlobStore,
	) -> Self {
		Self {
			events,
			rollups,
			archive,
			retention: default(),
			full: false,
		}
	}

	/// The windows the raws are stamped with once their day is covered.
	pub fn with_retention(mut self, retention: AnalyticsRetention) -> Self {
		self.retention = retention;
		self
	}

	/// Sweep EVERY complete day in the table rather than the uncovered ones: the
	/// one-time backfill, which re-archives and re-aggregates a history whose
	/// rows predate the pipeline.
	pub fn with_full(mut self, full: bool) -> Self {
		self.full = full;
		self
	}

	/// Run the pipeline, returning what it covered.
	pub async fn call(&self) -> Result<AnalyticsRollupReport> {
		let mut by_date = HashMap::<SmolStr, Vec<AnalyticsEvent>>::default();
		let mut scanned = 0;
		for (_, event) in self.events.get_all_lossy().await? {
			scanned += 1;
			by_date.entry(event.date()).or_default().push(event);
		}
		let dates = self.dates(&by_date).await?;
		let day =
			|date: &SmolStr| by_date.get(date).cloned().unwrap_or_default();

		// 1. the cold copy, first and always: nothing below may run for a day
		//    whose archive object is not confirmed present.
		let mut archived = Vec::new();
		for date in &dates {
			archived.push(SmolStr::from(
				AnalyticsArchive::write(&self.archive, date, &day(date))
					.await?
					.to_string(),
			));
		}

		// 2. the aggregates, which are what a report reads once the raws are
		//    gone, each confirmed before its day counts as covered.
		let aggregates = dates
			.iter()
			.flat_map(|date| {
				AnalyticsRollup::from_events(&day(date))
					.into_iter()
					.map(move |row| (date, row))
			})
			.map(async |(date, row)| -> Result {
				let id = row.id;
				self.rollups.push(row).await?;
				if !self.rollups.exists(id).await? {
					bevybail!(
						"the {date} aggregate was written to {} but is not there: \
						 nothing may expire until it is",
						self.rollups.describe()
					);
				}
				Ok(())
			});
		let rollups = Self::drive(aggregates).await?;

		// 3. and only now the expiry, on the covered days alone. A row the
		//    recorder already stamped is left as it is; one from before the
		//    pipeline gets a floor of `GRACE` so a botched sweep is readable in
		//    the morning rather than already deleted.
		let now = time_ext::now();
		let stamps = dates
			.iter()
			.flat_map(day)
			.filter(|event| event.ttl.is_none())
			.filter_map(|event| {
				self.retention
					.expires_at_with_grace(
						event.event_kind,
						Duration::from_millis(event.timestamp),
						now,
					)
					.map(|ttl| AnalyticsEvent {
						ttl: Some(ttl),
						..event
					})
			})
			.map(async |event| self.events.push(event).await);
		let expired = Self::drive(stamps).await?;

		AnalyticsRollupReport {
			full: self.full,
			scanned,
			dates,
			archived,
			rollups,
			expired,
		}
		.xok()
	}

	/// Run `writes` with the store fan-out bound, returning how many landed.
	///
	/// A backfill of a pre-pipeline history is hundreds of thousands of single
	/// row writes, and one round trip at a time does not finish inside a
	/// function's timeout. The bound is [`BlobStore::GET_ALL_CONCURRENCY`], the
	/// same one the whole-table read uses, so a run never opens more connections
	/// to a store than reading it already does.
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

	/// The days this run covers: every COMPLETE day the table holds, minus (on
	/// an ordinary run) the ones already both archived and aggregated.
	///
	/// Today is never covered, since it is still being written. Skipping only
	/// fully covered days is what makes the job self-healing: a week the
	/// schedule missed is picked up by the next run rather than expiring
	/// unarchived, which is the one way this pipeline could lose history.
	async fn dates(
		&self,
		by_date: &HashMap<SmolStr, Vec<AnalyticsEvent>>,
	) -> Result<Vec<SmolStr>> {
		let today = SmolStr::from(time_ext::format_date(time_ext::now()));
		let mut complete = by_date
			.keys()
			.filter(|date| **date < today)
			.cloned()
			.collect::<Vec<_>>();
		complete.sort();
		let mut dates = Vec::new();
		for date in complete {
			if self.full || !self.covered(&date).await? {
				dates.push(date);
			}
		}
		dates.xok()
	}

	/// Whether `date` already has both the things an expiry stamp depends on:
	/// its archive object and its site-wide aggregate.
	async fn covered(&self, date: &str) -> Result<bool> {
		Ok(self
			.archive
			.blob(AnalyticsArchive::object_path(date))
			.exists()
			.await? && self
			.rollups
			.exists(AnalyticsRollup::row_id(date, &AnalyticsScope::Site))
			.await?)
	}
}

/// What one [`AnalyticsRollupRun`] covered, the invoke's own report.
///
/// A scheduled job has no client waiting on its body, so this exists to land in
/// the logs: a run that archived nothing and stamped everything is the shape of
/// the failure the ordering above exists to prevent, and it should be readable
/// at a glance.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalyticsRollupReport {
	/// Whether this was a full backfill rather than an ordinary run.
	pub full: bool,
	/// Raw rows read out of the events table.
	pub scanned: u32,
	/// The days covered, ascending.
	pub dates: Vec<SmolStr>,
	/// The archive objects written and confirmed.
	pub archived: Vec<SmolStr>,
	/// Aggregate rows written.
	pub rollups: u32,
	/// Raw rows given an expiry they did not already carry.
	pub expired: u32,
}

impl core::fmt::Display for AnalyticsRollupReport {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		writeln!(
			f,
			"analytics {}: {} days from {} scanned rows",
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
		write!(f, "  expiring:   {} raw rows stamped", self.expired)
	}
}

/// Request params for [`AnalyticsRollupJob`], surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsRollupParams {
	/// Sweep every complete day in the table, rather than the days not yet
	/// archived and rolled up. The one-time backfill of a pre-pipeline history.
	full: Option<bool>,
}

/// Archive, aggregate and expire the analytics table, as a route.
///
/// The job a schedule invokes, and the only writer of [`AnalyticsRollup`] rows.
/// It names its three stores by relation, never by name: the events table it
/// reads and stamps ([`StoreRef`]), the aggregate table it writes
/// ([`RollupStoreRef`]) and the store it archives into ([`ArchiveStoreRef`]).
///
/// ```html
/// <Route path="rollup" {(
///   AnalyticsRollupJob,
///   StoreRef($analytics),
///   RollupStoreRef($rollup),
///   ArchiveStoreRef($runtime_ops),
/// )}/>
/// ```
///
/// A run is idempotent: aggregate ids are a pure function of the day they cover
/// and archive objects are named by it, so `--full` may be re-run at will.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsRollupParams>())]
pub async fn AnalyticsRollupJob(
	cx: ActionContext<Request>,
) -> Result<Response> {
	let caller = cx.caller.clone();
	let entity = caller.id();
	let world = caller.world().clone();
	let full = cx.input.request_parts().has_param("full");

	// the windows the raws are stamped with, declared once for the whole entry
	// and resolved here by ancestry, so the job and the recorder cannot drift.
	let retention = world
		.with_state::<AncestorQuery<&AnalyticsRetention>, AnalyticsRetention>(
			move |query| query.get(entity).copied().unwrap_or_default(),
		)
		.await;
	retention.validate()?;

	let declared = async |name: &str, target: Result<Entity>| match target {
		Ok(target) => Ok(target),
		Err(_) => bevybail!(
			"the analytics rollup job names the stores it works on: add \
			 `{name}($declaration)` beside it, pointing at the block that \
			 declares the store"
		),
	};
	let events = declared(
		"StoreRef",
		caller
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await,
	)
	.await?;
	let rollups = declared(
		"RollupStoreRef",
		caller.get::<RollupStoreRef, _>(|it| it.0).await,
	)
	.await?;
	let archive = declared(
		"ArchiveStoreRef",
		caller.get::<ArchiveStoreRef, _>(|it| it.0).await,
	)
	.await?;

	let report = AnalyticsRollupRun::new(
		StoreRef::resolve::<TableStore>(&world, events)
			.await?
			.table(),
		StoreRef::resolve::<TableStore>(&world, rollups)
			.await?
			.table(),
		StoreRef::resolve::<BlobStore>(&world, archive).await?,
	)
	.with_retention(retention)
	.with_full(full)
	.call()
	.await?;
	info!("{report}");
	Response::ok_text(report.to_string()).xok()
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_action::prelude::*;
	use beet_core::prelude::*;

	const DAY_MS: u64 = 86_400_000;

	/// A run over three temp stores, plus the stores themselves.
	fn run() -> (
		AnalyticsRollupRun,
		Table<AnalyticsEvent>,
		Table<AnalyticsRollup>,
		BlobStore,
	) {
		let events = Table::<AnalyticsEvent>::temp();
		let rollups = Table::<AnalyticsRollup>::temp();
		let archive = BlobStore::temp();
		(
			AnalyticsRollupRun::new(
				events.clone(),
				rollups.clone(),
				archive.clone(),
			),
			events,
			rollups,
			archive,
		)
	}

	/// A page view `days` ago, so every fixture sits safely in the past whatever
	/// day the suite runs on.
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

	/// The pipeline in order: every covered day is archived and aggregated
	/// before a single raw row is stamped, and both artifacts read back.
	#[beet_core::test]
	async fn archives_and_aggregates_before_expiring() {
		let (run, events, rollups, archive) = run();
		let sources = [
			page_view(3, "/", 1),
			page_view(3, "/docs", 1),
			page_view(2, "/", 2),
		];
		for event in &sources {
			events.push(event.clone()).await.unwrap();
		}
		let report = run.call().await.unwrap();
		report.scanned.xpect_eq(3);
		report.dates.len().xpect_eq(2);
		report.archived.len().xpect_eq(2);
		// site + / + /docs on the older day, site + / on the newer
		report.rollups.xpect_eq(5);
		report.expired.xpect_eq(3);

		// the archive holds the raws, losslessly
		let date = sources[0].date();
		AnalyticsArchive::decode(
			&archive
				.get(&AnalyticsArchive::object_path(&date))
				.await
				.unwrap(),
		)
		.unwrap()
		.len()
		.xpect_eq(2);
		// the aggregate holds the day, its site row deduping the session that
		// read two paths
		let site = rollups
			.get(AnalyticsRollup::row_id(&date, &AnalyticsScope::Site))
			.await
			.unwrap();
		site.views.xpect_eq(2);
		site.visits.xpect_eq(1);
		// ..and only now do the raws carry an expiry
		events.get(sources[0].id).await.unwrap().ttl.xpect_some();
	}

	/// Today is never covered: it is still being written, so archiving it would
	/// publish a partial day and stamping it would expire rows no aggregate has
	/// seen.
	#[beet_core::test]
	async fn leaves_today_alone() {
		let (run, events, ..) = run();
		let today = page_view(0, "/", 1);
		events.push(today.clone()).await.unwrap();
		let report = run.call().await.unwrap();
		report.dates.is_empty().xpect_true();
		report.expired.xpect_eq(0);
		events.get(today.id).await.unwrap().ttl.xpect_none();
	}

	/// An ordinary run skips a day already archived AND aggregated, and covers
	/// one missing either — so a week the schedule missed is picked up rather
	/// than expiring unarchived. A full run covers everything regardless.
	#[beet_core::test]
	async fn covers_what_is_not_yet_covered() {
		let (run, events, _rollups, archive) = run();
		let event = page_view(2, "/", 1);
		events.push(event.clone()).await.unwrap();
		run.call().await.unwrap().dates.len().xpect_eq(1);
		// ..a second run has nothing left to do
		run.call().await.unwrap().dates.is_empty().xpect_true();
		// ..but losing the archive object makes the day uncovered again
		archive
			.remove(&AnalyticsArchive::object_path(&event.date()))
			.await
			.unwrap();
		run.call().await.unwrap().dates.len().xpect_eq(1);
		// ..and a backfill sweeps a day that is already covered
		run.with_full(true)
			.call()
			.await
			.unwrap()
			.dates
			.len()
			.xpect_eq(1);
	}

	/// A backfilled row already past its window is stamped a grace period out,
	/// never an expiry in the past, and a row the recorder already stamped keeps
	/// the expiry it was given.
	#[beet_core::test]
	async fn backfilled_rows_get_their_grace() {
		let (run, events, ..) = run();
		let retention = AnalyticsRetention::default();
		let old = page_view(200, "/", 1);
		let stamped = page_view(1, "/", 2).with_retention(&retention);
		let already = stamped.ttl.unwrap();
		for event in [&old, &stamped] {
			events.push(event.clone()).await.unwrap();
		}
		run.with_retention(retention).call().await.unwrap();
		// the 200 day old row is 110 days past its 90 day window, so it takes
		// the grace floor rather than an expiry in the past
		let now = time_ext::now().as_secs();
		let ttl = events.get(old.id).await.unwrap().ttl.unwrap();
		ttl.xpect_greater_than(now);
		ttl.xpect_less_or_equal_to(
			now + AnalyticsRetention::GRACE.as_secs() + 5,
		);
		// the already-stamped row is untouched
		events
			.get(stamped.id)
			.await
			.unwrap()
			.ttl
			.xpect_eq(Some(already));
	}

	/// The route half: the job resolves all three stores through the relations
	/// it declares, never by name, and `--full` reaches the run.
	#[beet_core::test]
	async fn dispatches_over_its_declared_stores() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let stores = [(); 3].map(|_| world.spawn(InMemoryStore::new()).flush());
		let [events, rollups, archive] = stores;
		let event = page_view(2, "/docs", 1);
		world
			.entity(events)
			.get::<TableStore>()
			.unwrap()
			.table::<AnalyticsEvent>()
			.push(event.clone())
			.await
			.unwrap();
		let job = world
			.spawn((
				AnalyticsRollupJob::default(),
				StoreRef(events),
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
			.xpect_contains("archived:   1 objects");
		// the aggregate landed in the table the relation named, not the events one
		world
			.entity(rollups)
			.get::<TableStore>()
			.unwrap()
			.table::<AnalyticsRollup>()
			.get(AnalyticsRollup::row_id(
				&event.date(),
				&AnalyticsScope::Site,
			))
			.await
			.unwrap()
			.views
			.xpect_eq(1);
	}

	/// A job with nothing to point at fails naming the relation that would have
	/// pointed it somewhere, rather than falling back to a store nobody reads.
	#[beet_core::test]
	async fn an_unpointed_job_fails_loudly() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let store = world.spawn(InMemoryStore::new()).flush();
		// each relation is named by the failure of the job that lacks it
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

	/// A kind kept forever is never stamped, so a site can opt out of expiry
	/// entirely and still get its archive and aggregates.
	#[beet_core::test]
	async fn a_zero_window_expires_nothing() {
		let (run, events, ..) = run();
		let event = page_view(2, "/", 1);
		events.push(event.clone()).await.unwrap();
		let report = run
			.with_retention(AnalyticsRetention {
				requests: Duration::ZERO,
				events: Duration::ZERO,
			})
			.call()
			.await
			.unwrap();
		report.archived.len().xpect_eq(1);
		report.expired.xpect_eq(0);
		events.get(event.id).await.unwrap().ttl.xpect_none();
	}
}
