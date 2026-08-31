//! The [`AnalyticsStore`] backend and the config-triggered bootstrap +
//! persistence observers.
use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// Component holding the analytics table, spawned on the [`AnalyticsConfig`]
/// entity.
///
/// Writes are fire-and-forget off the request path, so a table that will not
/// answer would otherwise report one error per event forever: a wrong table name
/// once survived a whole deploy that way, logging a `ResourceNotFoundException`
/// per request while the site served perfectly. The first failure therefore
/// raises with the resolved target and the full error, then poisons the store so
/// every later write is a `debug!` line rather than another raise.
#[derive(Clone, Deref, Component)]
pub struct AnalyticsStore {
	/// The typed view over the resolved store entity's [`TableStore`].
	#[deref]
	pub store: Table<AnalyticsEvent>,
	/// Set by the first failed write. Shared, so a cloned handle in a detached
	/// task poisons the component the world still holds.
	poisoned: Arc<AtomicBool>,
}

impl AnalyticsStore {
	/// Create from a resolved typed table.
	pub fn new(store: Table<AnalyticsEvent>) -> Self {
		Self {
			store,
			poisoned: Arc::new(AtomicBool::new(false)),
		}
	}

	/// The local analytics store at `dir`, the same store a dev server derives.
	/// Used by `beet analytics` to query without a scene.
	#[cfg(feature = "fs")]
	pub fn local(dir: AbsPathBuf) -> Self { Self::new(Table::local(dir)) }

	/// The remote (DynamoDB) analytics store at `table_name`, in whichever region
	/// the SDK's default provider chain resolves. Used by `beet analytics
	/// --remote` to query without a scene (there is no declaration to read a
	/// region off); errors without the `aws_sdk` backend.
	pub fn remote(table_name: &str) -> Result<Self> {
		Table::remote(table_name).map(Self::new)
	}

	/// Record `event`, raising exactly once for a store that will not answer.
	///
	/// The first failure carries the resolved table, its region and the whole
	/// error chain through the app's configured handler (which panics on a debug
	/// build so a misconfiguration surfaces immediately, and logs on release so
	/// the site keeps serving). Every later event is dropped at `debug!`.
	pub async fn record(&self, event: AnalyticsEvent) -> Result {
		if self.poisoned.load(Ordering::Relaxed) {
			debug!(
				"analytics: dropping {:?} event for {}, {} is not answering",
				event.event_kind,
				event.path,
				self.store.describe()
			);
			return Ok(());
		}
		match self.store.push(event).await {
			Ok(()) => Ok(()),
			Err(err) => {
				self.poisoned.store(true, Ordering::Relaxed);
				bevybail!(
					"analytics: could not record to {}, every later event is dropped.\n{err}",
					self.store.describe()
				)
			}
		}
	}
}

/// Observer: on an [`AnalyticsConfig`] insertion, resolve the declared store and
/// insert the [`AnalyticsStore`] on the config's own entity.
///
/// Declaration-first: a [`StoreRef`] on the config entity names the entity
/// that *declares* the store (a `<DynamoTableBlock/>`, an `<FsStore/>`, any
/// store provider), so the deploy that creates a table and the runtime that
/// writes to it read one declaration and can never resolve different names.
///
/// Config-triggered rather than a startup system so it is inert until analytics
/// is switched on, and so it works whenever the config lands (markup scenes
/// resolve asynchronously). Idempotent, so a scene reload does not rebuild the
/// store.
pub(super) fn spawn_store_on_config(
	ev: On<Add, AnalyticsConfig>,
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	if stores.contains(ev.entity) {
		return;
	}
	let entity = ev.entity;
	commands.run(async move |world| {
		let config_entity = world.entity(entity);
		// guard against a racing second insertion creating it first.
		if config_entity.get::<AnalyticsStore, _>(|_| ()).await.is_ok() {
			return Ok(());
		}
		// the store this config records to, declared once and named here.
		let Ok(target) = config_entity
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await
		else {
			bevybail!(
				"an `AnalyticsConfig` records to the store it names: add a `StoreRef` beside it pointing at a store declaration, ie `<DynamoTableBlock bx:ref=\"analytics\" label=\"analytics\"/>` and `{{(AnalyticsConfig, StoreRef($analytics))}}`"
			);
		};
		// the provider hooks insert the TableStore through the command queue,
		// so the resolve gives it a settle window before erroring.
		let table_store =
			StoreRef::resolve::<TableStore>(&world, target).await?;
		config_entity
			.insert(AnalyticsStore::new(table_store.table()))
			.await?;
		Ok(())
	});
}

/// Observer: persist a triggered [`AnalyticsEvent`] to the store each
/// [`AnalyticsConfig`] declared.
///
/// The single sink for every emitter (request middleware, navigator, web
/// beacon). No config (analytics not switched on) drops the event rather than
/// panicking, since emitters trigger unconditionally. Fire-and-forget: the push
/// runs on the async queue so recording never blocks a request or a navigation.
///
/// Scoped to [`AnalyticsConfig`] entities rather than to every
/// [`AnalyticsStore`] in the world, so a world holding two routers records each
/// one's traffic to the store that router declared (through its
/// [`StoreRef`], resolved once by [`spawn_store_on_config`]) and to no
/// other.
///
/// Recording is also where an event's expiry is stamped, from the
/// [`AnalyticsRetention`] that config resolves by ancestry: the window belongs
/// to the store being written to, so two routers recording to two stores each
/// stamp their own.
pub(super) fn handle_analytics_event(
	ev: On<AnalyticsEvent>,
	configs: Query<(Entity, &AnalyticsStore), With<AnalyticsConfig>>,
	retentions: AncestorQuery<&AnalyticsRetention>,
	commands: AsyncCommands,
) {
	let stores = configs
		.iter()
		.map(|(entity, store)| {
			(
				store.clone(),
				retentions.get(entity).copied().unwrap_or_default(),
			)
		})
		.collect::<Vec<_>>();
	if stores.is_empty() {
		return;
	}
	let event = ev.event().clone();
	commands.run(async move |_| {
		for (store, retention) in stores {
			store
				.record(event.clone().with_retention(&retention))
				.await?;
		}
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use crate::exports::bytes::Bytes;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use std::sync::Arc;
	use std::sync::atomic::AtomicUsize;
	use std::sync::atomic::Ordering;

	/// A router-shaped world: an analytics config bound to a declared store.
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

	#[beet_core::test]
	async fn event_roundtrips_through_store() {
		let store = Table::<AnalyticsEvent>::temp();
		let event =
			AnalyticsEvent::new("/about", AnalyticsEventData::Request {
				status: 200,
				method: "GET".into(),
				user_agent: None,
				referrer: None,
			})
			.with_client_kind(ClientKind::Web);
		let id = event.id;
		store.push(event).await.unwrap();
		let loaded = store.get(id).await.unwrap();
		loaded.path.as_str().xpect_eq("/about");
		loaded.event_kind.xpect_eq(AnalyticsEventKind::Request);
	}

	/// The config's own entity carries the store, so consumers resolve it by
	/// ancestry rather than as a global, and the table itself lives on the
	/// declaration the config names.
	///
	/// The country database is a separate declaration, so a config alone carries
	/// no [`GeoIp`]: the store must not wait on an ~8 MB mmdb read to start
	/// recording.
	#[beet_core::test]
	async fn config_entity_carries_the_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (config, _) = config_with_store(&mut world, InMemoryStore::new());
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.entity(config)
			.contains::<AnalyticsStore>()
			.xpect_true();
		world.entity(config).contains::<GeoIp>().xpect_false();
	}

	/// The site's real shape: a router carrying the config, the store it records
	/// to and its country database, under an entry that owns the blob store.
	///
	/// The database is *content*, so it resolves out of that ancestor
	/// [`BlobStore`] rather than through a relation, and lands on its own
	/// declaration entity where consumers still find it by ancestry. It is also
	/// wholly independent of the store: the [`AnalyticsStore`] lands whatever the
	/// database does, so an ~8 MB mmdb read never gates the first recorded event,
	/// and a database that is not in the store degrades to empty lookups rather
	/// than failing the load.
	#[beet_core::test]
	async fn geoip_db_is_content() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let table = world.spawn(InMemoryStore::new()).flush();
		let entry = world.spawn(InMemoryStore::new()).flush();
		let router = world
			.spawn((
				ChildOf(entry),
				AnalyticsConfig::default(),
				StoreRef(table),
				GeoIpDb::default(),
			))
			.flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world
			.entity(router)
			.contains::<AnalyticsStore>()
			.xpect_true();
		world
			.entity(router)
			.get::<GeoIp>()
			.unwrap()
			.country_str("8.8.8.8")
			.xpect_none();
	}

	/// The declared store is where the events land, resolved through the
	/// relation rather than derived from a naming convention.
	#[beet_core::test]
	async fn records_to_the_declared_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let (_, store_entity) =
			config_with_store(&mut world, InMemoryStore::new());
		AsyncRunner::settle_async_tasks(&mut world).await;

		let event =
			AnalyticsEvent::new("/about", AnalyticsEventData::Request {
				status: 200,
				method: "GET".into(),
				user_agent: None,
				referrer: None,
			});
		let id = event.id;
		world.trigger(event);
		AsyncRunner::settle_async_tasks(&mut world).await;

		world
			.entity(store_entity)
			.get::<TableStore>()
			.unwrap()
			.table::<AnalyticsEvent>()
			.get(id)
			.await
			.unwrap()
			.path
			.as_str()
			.xpect_eq("/about");
	}

	/// A recorded event carries the expiry its router's [`AnalyticsRetention`]
	/// gives its kind, resolved by ancestry so one declaration covers the whole
	/// subtree, and requests expire ahead of the streams a client reports.
	///
	/// This is the LAST step of the retention order, never the first: the row is
	/// archived cold and rolled up into a daily aggregate weeks before the stamp
	/// here comes due.
	#[beet_core::test]
	async fn records_stamp_their_retention() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let store = world.spawn(InMemoryStore::new()).flush();
		let entry = world
			.spawn(AnalyticsRetention {
				requests: Duration::from_secs(10 * 86_400),
				events: Duration::from_secs(20 * 86_400),
			})
			.flush();
		world
			.spawn((
				ChildOf(entry),
				AnalyticsConfig::default(),
				StoreRef(store),
			))
			.flush();
		AsyncRunner::settle_async_tasks(&mut world).await;

		for (data, days) in [
			(
				AnalyticsEventData::Request {
					status: 200,
					method: "GET".into(),
					user_agent: None,
					referrer: None,
				},
				10,
			),
			(AnalyticsEventData::Scroll { max_percent: 50 }, 20),
		] {
			let event = AnalyticsEvent::new("/about", data);
			let (id, recorded) = (event.id, event.timestamp);
			world.trigger(event);
			AsyncRunner::settle_async_tasks(&mut world).await;
			world
				.entity(store)
				.get::<TableStore>()
				.unwrap()
				.table::<AnalyticsEvent>()
				.get(id)
				.await
				.unwrap()
				.ttl
				.xpect_eq(Some(recorded / 1000 + days * 86_400));
		}
	}

	/// Everything raised through the collecting handler below. A `static`
	/// because bevy's error handler is a bare fn pointer with nowhere to capture
	/// state; only [`analytics_failures_are_loud`] routes a world into it, so
	/// there is one writer.
	static RAISED: std::sync::Mutex<Vec<String>> =
		std::sync::Mutex::new(Vec::new());

	fn collect_error(err: BevyError, _cx: bevy::ecs::error::ErrorContext) {
		RAISED.lock().unwrap().push(err.to_string());
	}

	/// A world whose raised errors land in [`RAISED`], which starts empty.
	fn collecting_world() -> World {
		RAISED.lock().unwrap().clear();
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		world.insert_resource(bevy::ecs::error::FallbackErrorHandler(
			collect_error,
		));
		world
	}

	fn raised() -> Vec<String> { RAISED.lock().unwrap().clone() }

	/// Both loud paths, in one test because they share the one static collector.
	///
	/// 1. A config that names no store raises at spawn time rather than falling
	///    back to a directory nobody reads.
	/// 2. A store that will not answer raises ONCE and is then poisoned. The
	///    original incident survived a whole deploy precisely because it
	///    produced a silent error per event while the site served perfectly.
	#[beet_core::test]
	async fn analytics_failures_are_loud() {
		let mut world = collecting_world();
		world.spawn(AnalyticsConfig::default()).flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		raised().join("\n").xpect_contains("StoreRef");

		let mut world = collecting_world();
		let attempts = Arc::new(AtomicUsize::new(0));
		config_with_store(&mut world, FailingStore {
			attempts: attempts.clone(),
		});
		AsyncRunner::settle_async_tasks(&mut world).await;
		for index in 0..8 {
			world.trigger(AnalyticsEvent::new(
				format!("/page-{index}"),
				AnalyticsEventData::Request {
					status: 200,
					method: "GET".into(),
					user_agent: None,
					referrer: None,
				},
			));
			AsyncRunner::settle_async_tasks(&mut world).await;
		}
		// one raise, naming the table that would not answer..
		let raised = raised();
		raised.len().xpect_eq(1);
		raised[0]
			.as_str()
			.xpect_contains("could not record to")
			.xpect_contains("no-such-table");
		// ..and the store is not hammered once poisoned.
		attempts.load(Ordering::SeqCst).xpect_eq(1);
	}

	/// A store whose every write fails, standing in for a table that does not
	/// exist.
	#[derive(Clone, Component)]
	#[component(on_add = BlobStore::on_add::<Self>)]
	struct FailingStore {
		attempts: Arc<AtomicUsize>,
	}

	impl BlobStoreProvider for FailingStore {
		fn box_clone(&self) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn with_subdir(&self, _path: SmolPath) -> Box<dyn BlobStoreProvider> {
			Box::new(self.clone())
		}
		fn id(&self) -> &'static str { "failing" }
		fn root_key(&self) -> SmolStr { "no-such-table".into() }
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
		fn insert(
			&self,
			_path: &SmolPath,
			_body: Bytes,
		) -> SendBoxedFuture<Result> {
			self.attempts.fetch_add(1, Ordering::SeqCst);
			Box::pin(async { bevybail!("ResourceNotFoundException") })
		}
		fn list(&self) -> SendBoxedFuture<Result<Vec<SmolPath>>> {
			Box::pin(async { Vec::new().xok() })
		}
		fn get(&self, path: &SmolPath) -> SendBoxedFuture<Result<Bytes>> {
			let path = path.clone();
			Box::pin(async move { bevybail!("no row at {path}") })
		}
		fn exists(&self, _path: &SmolPath) -> SendBoxedFuture<Result<bool>> {
			Box::pin(async { false.xok() })
		}
		fn remove(&self, _path: &SmolPath) -> SendBoxedFuture<Result> {
			Box::pin(async { Ok(()) })
		}
		fn public_url(
			&self,
			_path: &SmolPath,
		) -> SendBoxedFuture<Result<Option<String>>> {
			Box::pin(async { None.xok() })
		}
	}
}
