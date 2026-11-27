//! Analytics persistence: each event a row the moment it happens.
use crate::prelude::*;
use beet_core::prelude::*;

/// The analytics tables of one [`TableStore`]: raw events in
/// `analytics_event`, one row each, written as they happen with no buffer,
/// and their daily aggregates in `analytics_rollup`.
///
/// Every writer writes its own rows, so any number of processes record at
/// once, a serverless one evicted without notice included, and nothing is
/// lost at exit that was recorded before it.
#[derive(Clone, Component)]
pub struct AnalyticsStore {
	store: TableStore,
}

impl AnalyticsStore {
	/// How many fresh keys a write tries before giving up on a collision,
	/// which needs two writers to draw the same of a million keys in one
	/// millisecond.
	const KEY_ATTEMPTS: usize = 4;

	/// The analytics tables of `store`.
	pub fn new(store: TableStore) -> Self { Self { store } }

	/// The store the tables live in.
	pub fn store(&self) -> &TableStore { &self.store }

	/// The raw events.
	pub fn events(&self) -> Table<AnalyticsEvent> { self.store.table() }

	/// The daily aggregates.
	pub fn rollups(&self) -> Table<AnalyticsRollup> { self.store.table() }

	/// Where the tables live, for any error naming them.
	pub fn describe(&self) -> String { self.store.describe() }

	/// Write `event` as a new row, retrying a key another writer took.
	pub async fn record(&self, mut event: AnalyticsEvent) -> Result {
		let events = self.events();
		for _ in 0..Self::KEY_ATTEMPTS {
			match events.push(event.clone()).await {
				Ok(_) => return Ok(()),
				Err(err) if TableError::is_conflict(&err) => {
					event = event.with_fresh_key();
				}
				Err(err) => {
					bevybail!(
						"analytics: could not record an event to {}: {err}",
						self.describe()
					)
				}
			}
		}
		bevybail!(
			"analytics: {} keys collided recording an event to {}",
			Self::KEY_ATTEMPTS,
			self.describe()
		)
	}

	/// Every event recorded on `date`, a page view's heartbeats reduced to
	/// its last.
	pub async fn day(&self, date: Date) -> Result<Vec<AnalyticsEvent>> {
		self.events()
			.scan_all(AnalyticsEvent::day_scan(date))
			.await?
			.xmap(AnalyticsEvent::dedupe)
			.xok()
	}

	/// The day of the earliest event still in the table, `None` when it is
	/// empty.
	pub async fn first_day(&self) -> Result<Option<Date>> {
		self.events()
			.scan(Scan::default().limit(1))
			.await?
			.rows
			.first()
			.map(|event| Date::from(event.timestamp()))
			.xok()
	}

	/// The latest day rolled up whole, `None` before the first rollup: the
	/// last day with aggregates when its site-wide row, written last, is
	/// among them, else the day before it, the last one having been cut short.
	pub async fn last_rolled_day(&self) -> Result<Option<Date>> {
		let rollups = self.rollups();
		let page = rollups.scan(Scan::default().reverse().limit(1)).await?;
		let Some(last) = page.rows.first() else {
			return None.xok();
		};
		let date = Date::parse(&last.date)?;
		match rollups.exists(Self::site_key(date)).await? {
			true => date,
			false => date.add_days(-1),
		}
		.xsome()
		.xok()
	}

	/// Compose the [`AnalyticsSummary`] of everything recorded: the daily
	/// aggregates for the days they cover whole, and the raw events of the
	/// days after the last of them. `raw_only` reads the raw events alone. An
	/// unreadable row is skipped with a warning rather than failing the
	/// summary, and a store never written to summarizes as empty.
	pub async fn summary(&self, raw_only: bool) -> Result<AnalyticsSummary> {
		let last = match raw_only {
			true => None,
			false => self.last_rolled_day().await?,
		};
		let rollups = match last {
			Some(last) => self
				.rollups()
				.get_all_lossy()
				.await?
				.into_iter()
				.filter(|row| {
					Date::parse(&row.date).is_ok_and(|date| date <= last)
				})
				.collect(),
			None => Vec::new(),
		};
		let events = self.events_after(last).await?;
		AnalyticsSummary::compose(&rollups, &events).xok()
	}

	/// The visits of everything recorded, [`summary`](Self::summary)'s count
	/// read from the site-wide aggregates alone, one key a day: what a page
	/// showing it can afford on every render.
	pub async fn visits(&self) -> Result<usize> {
		let last = self.last_rolled_day().await?;
		let rollups = self.rollups();
		let first = rollups.scan(Scan::default().limit(1)).await?;
		let days = match (first.rows.first(), last) {
			(Some(first), Some(last)) => {
				let first = Date::parse(&first.date)?;
				(0..=first.days_until(last))
					.map(|days| first.add_days(days))
					.collect()
			}
			_ => Vec::new(),
		};
		let sites = async_ext::try_join_all_bounded(
			BlobStore::GET_ALL_CONCURRENCY,
			days.into_iter()
				.map(|date| rollups.get(Self::site_key(date))),
		)
		.await?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();
		let events = self.events_after(last).await?;
		AnalyticsSummary::compose(&sites, &events).visits.xok()
	}

	/// The key of `date`'s site-wide aggregate.
	fn site_key(date: Date) -> TableKey {
		AnalyticsRollup::new(date.to_string(), AnalyticsScope::Site).key()
	}

	/// Every event after `last`, all of them without one, a page view's
	/// heartbeats reduced to its last and an unreadable row skipped with a
	/// warning.
	async fn events_after(
		&self,
		last: Option<Date>,
	) -> Result<Vec<AnalyticsEvent>> {
		let scan = match last {
			Some(last) => {
				Scan::default().from(Tid::from(last.add_days(1).timestamp()))
			}
			None => Scan::default(),
		};
		self.events()
			.scan_stored(scan)
			.await?
			.into_iter()
			.filter_map(|row| {
				let (key, _, row) = row.into_parts();
				AnalyticsEvent::from_row(&key, row)
					.map_err(|err| {
						warn!(
							"skipping unreadable analytics event {key}: {err}"
						)
					})
					.ok()
			})
			.xmap(AnalyticsEvent::dedupe)
			.xok()
	}
}

/// Observer: resolve the [`TableStore`] named beside an [`AnalyticsConfig`]
/// and attach its [`AnalyticsStore`] to that config entity.
pub(super) fn spawn_store_on_config(
	ev: On<Add<AnalyticsConfig>>,
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	if stores.contains(ev.entity) {
		return;
	}
	let entity = ev.entity;
	commands.run(async move |world| {
		let config_entity = world.entity(entity);
		if config_entity.get::<AnalyticsStore, _>(|_| ()).await.is_ok() {
			return Ok(());
		}
		let Ok(target) = config_entity
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await
		else {
			bevybail!(
				"an `AnalyticsConfig` records to the store it names: add a `StoreRef` beside it pointing at a store declaration, ie `<DynamoTableBlock bx:ref=\"analytics\" label=\"analytics\"/>` and `{{(AnalyticsConfig, StoreRef($analytics))}}`"
			);
		};
		let store = TableStore::declared(
			&world,
			target,
			"an `AnalyticsConfig`",
		)
		.await?;
		config_entity.insert(AnalyticsStore::new(store)).await?;
		Ok(())
	});
}

/// Observer: write every emitted event to each configured store.
pub(super) fn handle_analytics_event(
	ev: On<AnalyticsEvent>,
	stores: Query<&AnalyticsStore, With<AnalyticsConfig>>,
	commands: AsyncCommands,
) {
	let stores = stores.iter().cloned().collect::<Vec<_>>();
	if stores.is_empty() {
		return;
	}
	let event = ev.event().clone();
	commands.run(async move |_| {
		for store in stores {
			store.record(event.clone()).await?;
		}
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::exports::bytes::Bytes;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	fn request(path: &str) -> AnalyticsEvent {
		AnalyticsEvent::new(path, AnalyticsEventData::Request {
			status: 200,
			method: "GET".into(),
			user_agent: None,
			referrer: None,
		})
	}

	fn config_with_store(
		world: &mut World,
		store: impl Bundle,
	) -> (Entity, Entity) {
		let store = world.spawn(store).flush();
		let config = world
			.spawn((AnalyticsConfig::default(), StoreRef(store)))
			.flush();
		(config, store)
	}

	/// An event is a row the moment it is recorded, read back by its day.
	#[beet_core::test]
	async fn records_a_row() {
		let analytics =
			AnalyticsStore::new(TableStore::new(InMemoryTableStore::new()));
		let event = request("/about");
		let id = event.id;
		analytics.record(event).await.unwrap();
		let loaded = analytics.day(Date::today()).await.unwrap();
		loaded.len().xpect_eq(1);
		loaded[0].id.xpect_eq(id);
		loaded[0].path.as_str().xpect_eq("/about");
		analytics
			.first_day()
			.await
			.unwrap()
			.xpect_eq(Some(Date::today()));
	}

	/// A key another writer took is retried with a fresh one, so both
	/// events land.
	#[beet_core::test]
	async fn retries_a_taken_key() {
		let analytics =
			AnalyticsStore::new(TableStore::new(InMemoryTableStore::new()));
		let first = request("/first");
		let mut second = request("/second");
		second.key = first.key;
		analytics.record(first).await.unwrap();
		analytics.record(second).await.unwrap();
		analytics.events().list().await.unwrap().len().xpect_eq(2);
	}

	/// A page view's heartbeats share its id and keep only the newest.
	#[beet_core::test]
	async fn heartbeats_dedupe() {
		let analytics =
			AnalyticsStore::new(TableStore::new(InMemoryTableStore::new()));
		let first = request("/view");
		let mut later = first
			.clone()
			.with_timestamp(first.timestamp() + Duration::from_millis(1));
		later.path = "/view-later".into();
		analytics.record(first).await.unwrap();
		analytics.record(later).await.unwrap();
		let day = analytics.day(Date::today()).await.unwrap();
		day.len().xpect_eq(1);
		day[0].path.as_str().xpect_eq("/view-later");
	}

	/// A summary reads the aggregates for the days they cover and the raw
	/// events after them, each day from one source.
	#[beet_core::test]
	async fn summary_composes_rollups_and_events() {
		let analytics =
			AnalyticsStore::new(TableStore::new(InMemoryTableStore::new()));
		analytics.record(request("/today")).await.unwrap();
		let old = request("/old")
			.with_timestamp(Timestamp::now() - Duration::from_secs(2 * 86_400));
		for row in AnalyticsRollup::from_events(&[old.clone()]) {
			analytics.rollups().put(row).await.unwrap();
		}
		// the old day's raw event, already rolled, is not counted twice
		analytics.record(old).await.unwrap();
		analytics.summary(false).await.unwrap().requests.xpect_eq(2);
		analytics.summary(true).await.unwrap().requests.xpect_eq(2);
	}

	#[beet_core::test]
	async fn config_entity_carries_the_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (config, _) = config_with_store(
			&mut world,
			TableStore::new(InMemoryTableStore::new()),
		);
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.entity(config)
			.contains::<AnalyticsStore>()
			.xpect_true();
	}

	#[beet_core::test]
	async fn records_to_the_declared_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (_, store_entity) = config_with_store(
			&mut world,
			TableStore::new(InMemoryTableStore::new()),
		);
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.trigger(request("/about"));
		AsyncRunner::settle_async_tasks(&mut world).await;
		let store = world
			.entity(store_entity)
			.get::<TableStore>()
			.unwrap()
			.clone();
		AnalyticsStore::new(store).day(Date::today()).await.unwrap()[0]
			.path
			.as_str()
			.xpect_eq("/about");
	}

	static RAISED: std::sync::Mutex<Vec<String>> =
		std::sync::Mutex::new(Vec::new());

	fn collect_error(err: BevyError, _cx: bevy::ecs::error::ErrorContext) {
		RAISED.lock().unwrap().push(err.to_string());
	}

	fn collecting_world() -> World {
		RAISED.lock().unwrap().clear();
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		world.insert_resource(bevy::ecs::error::FallbackErrorHandler(
			collect_error,
		));
		world
	}

	fn raised() -> Vec<String> { RAISED.lock().unwrap().clone() }

	#[beet_core::test]
	async fn analytics_failures_are_loud() {
		let mut world = collecting_world();
		world.spawn(AnalyticsConfig::default()).flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		raised().join("\n").xpect_contains("StoreRef");

		let mut world = collecting_world();
		let attempts = Arc::new(AtomicUsize::new(0));
		config_with_store(
			&mut world,
			JsonTableStore::over(BlobStore::new(FailingStore {
				attempts: attempts.clone(),
			}))
			.unwrap(),
		);
		AsyncRunner::settle_async_tasks(&mut world).await;
		for index in 0..8 {
			world.trigger(request(&format!("/page-{index}")));
			AsyncRunner::settle_async_tasks(&mut world).await;
		}
		let raised = raised();
		raised.len().xpect_eq(8);
		raised[0]
			.as_str()
			.xpect_contains("could not record an event to")
			.xpect_contains("no-such-store");
		attempts.load(Ordering::SeqCst).xpect_eq(8);
	}

	#[derive(Clone)]
	struct FailingStore {
		attempts: Arc<AtomicUsize>,
	}

	impl StoreBackend for FailingStore {
		fn id(&self) -> &'static str { "failing" }

		fn describe(&self) -> String { self.root_key().to_string() }

		fn region(&self) -> Option<String> { None }

		fn store_exists(&self) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { true.xok() })
		}

		fn store_create(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}

		fn store_remove(&self) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
	}

	impl BlobProvider for FailingStore {
		fn box_clone(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }
		fn with_subdir(&self, _path: FwdPath) -> Box<dyn BlobProvider> {
			Box::new(self.clone())
		}
		fn base(&self) -> Box<dyn BlobProvider> { Box::new(self.clone()) }
		fn root_key(&self) -> SmolStr { "no-such-store".into() }
		fn insert(
			&self,
			_path: &FwdPath,
			_body: Bytes,
		) -> SendBoxedFuture<Result> {
			Box::pin(async { bevybail!("store unavailable") })
		}
		fn conditional_writes(&self) -> bool { true }

		fn get_versioned(
			&self,
			_path: &FwdPath,
		) -> SendBoxedFuture<Result<Option<(Bytes, BlobVersion)>>> {
			Box::pin(async { None.xok() })
		}

		fn insert_if(
			&self,
			_path: &FwdPath,
			_body: Bytes,
			_condition: BlobCondition,
		) -> SendBoxedFuture<Result<BlobVersion>> {
			self.attempts.fetch_add(1, Ordering::SeqCst);
			Box::pin(async { bevybail!("store unavailable") })
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<FwdPath>>> {
			Box::pin(async { Vec::new().xok() })
		}
		fn get(&self, path: &FwdPath) -> SendBoxedFuture<Result<Bytes>> {
			let path = path.clone();
			Box::pin(async move { bevybail!("no object at {path}") })
		}
		fn exists(&self, _path: &FwdPath) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { false.xok() })
		}
		fn remove(&self, _path: &FwdPath) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn public_url(
			&self,
			_path: &FwdPath,
		) -> SendBoxedFuture<Result<Option<String>>> {
			Box::pin(async { None.xok() })
		}
	}
}
