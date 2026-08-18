//! The [`AnalyticsStore`] backend and the config-triggered bootstrap +
//! persistence observers.
use crate::prelude::*;
use beet_core::prelude::*;

/// Component holding the analytics table, spawned on the [`AnalyticsConfig`]
/// entity.
#[derive(Clone, Deref, DerefMut, Component)]
pub struct AnalyticsStore {
	/// The typed view over the resolved store entity's [`TableStore`].
	pub store: Table<AnalyticsEvent>,
}

impl AnalyticsStore {
	/// Create from a resolved typed table.
	pub fn new(store: Table<AnalyticsEvent>) -> Self { Self { store } }

	/// The local analytics store at `dir`, the same store a dev server derives.
	/// Used by `beet analytics` to query without a scene.
	pub fn local(dir: AbsPathBuf) -> Self {
		Self::new(Table::new(BlobStore::new(FsStore::new(dir))))
	}

	/// The remote (DynamoDB) analytics store at `table_name`, in `AWS_REGION`
	/// else `us-west-2`. Used by `beet analytics --remote` to query without a
	/// scene; errors without the `aws_sdk` backend.
	pub fn remote(table_name: &str) -> Result<Self> {
		cfg_if! {
			if #[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))] {
				Self::new(Table::new(DynamoStore::new(
					table_name,
					DynamoStore::env_region(),
				)))
				.xok()
			} else {
				let _ = table_name;
				bevybail!("a remote analytics store requires the `aws_sdk` feature")
			}
		}
	}
}

/// Observer: on an [`AnalyticsConfig`] insertion, resolve the analytics store
/// entity and insert the [`AnalyticsStore`] and (under `geoip`) the country
/// database on the config's own entity.
///
/// Entity-first: [`AnalyticsConfig::store`] names an entity whose [`TableStore`]
/// backs the analytics table (any store provider component materializes one),
/// so any [`TableProvider`] backend plugs in without analytics naming it.
/// `None` spawns a convention-derived child store entity, resolved through the
/// same path.
///
/// Config-triggered rather than a startup system so it is inert until analytics
/// is switched on, and so it works whenever the config lands (markup scenes
/// resolve asynchronously). Reads the backend config inside the async task, so a
/// [`PackageConfig`] spawned alongside the [`AnalyticsConfig`] is already
/// present; idempotent, so a scene reload does not rebuild the store.
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
		// resolve the store entity: the configured reference, else a child
		// spawned with the convention-derived provider component.
		let target = config_entity
			.get::<AnalyticsConfig, _>(|config| config.store)
			.await?;
		let target = match target {
			Some(target) => target,
			None => {
				world
					.with(move |world: &mut World| {
						derived_store_entity(world, entity)
					})
					.await
			}
		};
		// the provider hooks insert the TableStore through the command queue,
		// so give it a settle window before erroring.
		let mut backoff = Backoff::default().with_max_attempts(5).stream();
		let table_store = loop {
			if let Ok(store) = world
				.entity(target)
				.get::<TableStore, _>(|store| store.clone())
				.await
			{
				break store;
			}
			if backoff.next().await.is_none() {
				bevybail!(
					"analytics store entity {target} has no TableStore: insert a store provider component, ie `<FsStore/>`"
				);
			}
		};
		// the offline country database rides the app's own store (the checkout in
		// dev, the app bucket when deployed), resolved like any other tree
		// consumer. Best-effort: no store, or no blob, just disables lookups.
		let blobs = config_entity
			.with_state::<AncestorQuery<&BlobStore>, _>(|entity, stores| {
				stores.get(entity).cloned().ok()
			})
			.await?;
		let geoip = match blobs {
			Some(blobs) => GeoIp::load(&blobs).await,
			None => GeoIp::default(),
		};
		config_entity
			.insert((AnalyticsStore::new(table_store.table()), geoip))
			.await?;
		Ok(())
	});
}

/// Spawn the convention-derived analytics store entity as a child of the
/// config entity: a local run writes [`WorkspaceConfig::analytics_dir`]; a
/// deployed ([`ServiceAccess::Remote`]) process uses the
/// `<app>--<stage>--analytics` DynamoDB table the deploy also derives
/// ([`PackageConfig::resource_name`]), in `AWS_REGION` else `us-west-2`.
/// Remote without the `aws_sdk` backend falls back to the local directory.
fn derived_store_entity(world: &mut World, config_entity: Entity) -> Entity {
	let access = BootstrapConfig::get().service_access;
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	if access == ServiceAccess::Remote {
		let table_name = world
			.get_resource::<PackageConfig>()
			.cloned()
			.unwrap_or_default()
			.analytics_bucket_name();
		return world
			.spawn((
				ChildOf(config_entity),
				DynamoStore::new(table_name, DynamoStore::env_region()),
			))
			.id();
	}
	#[cfg(not(all(feature = "aws_sdk", not(target_arch = "wasm32"))))]
	if access == ServiceAccess::Remote {
		debug!(
			"analytics: no `aws_sdk` backend, a remote store falls back to the local directory"
		);
	}
	let dir = world
		.get_resource::<WorkspaceConfig>()
		.cloned()
		.unwrap_or_default()
		.analytics_dir
		.into_abs();
	world
		.spawn((ChildOf(config_entity), FsStore::new(dir)))
		.id()
}

/// Observer: persist a triggered [`AnalyticsEvent`] to every [`AnalyticsStore`].
///
/// The single sink for every emitter (request middleware, navigator, web
/// beacon). No store (analytics not switched on) drops the event rather than
/// panicking, since emitters trigger unconditionally. Fire-and-forget: the push
/// runs on the async queue so recording never blocks a request or a navigation.
pub(super) fn handle_analytics_event(
	ev: On<AnalyticsEvent>,
	stores: Query<&AnalyticsStore>,
	commands: AsyncCommands,
) {
	let stores = stores.iter().cloned().collect::<Vec<_>>();
	if stores.is_empty() {
		return;
	}
	let event = ev.event().clone();
	commands.run(async move |_| {
		for store in stores {
			store.push(event.clone()).await?;
		}
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn event_roundtrips_through_store() {
		let store = Table::<AnalyticsEvent>::temp();
		let event = AnalyticsEvent::new("/about", AnalyticsEventData::Request {
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

	/// The config's own entity carries the store and the geoip component, so
	/// consumers resolve them by ancestry rather than as globals, and the
	/// convention-derived store entity is spawned as its child.
	#[beet_core::test]
	async fn config_entity_carries_the_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let entity = world.spawn(AnalyticsConfig::default()).flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.entity(entity).contains::<AnalyticsStore>().xpect_true();
		world.entity(entity).contains::<GeoIp>().xpect_true();
		world
			.entity(entity)
			.get::<Children>()
			.unwrap()
			.iter()
			.any(|child| world.entity(child).contains::<TableStore>())
			.xpect_true();
	}

	/// An explicit [`AnalyticsConfig::store`] reference persists events into
	/// that entity's [`TableStore`] rather than a derived one.
	#[beet_core::test]
	async fn explicit_store_entity() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let store_entity = world.spawn(InMemoryStore::new()).flush();
		let config = world
			.spawn(AnalyticsConfig {
				store: Some(store_entity),
				..default()
			})
			.flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.entity(config).contains::<AnalyticsStore>().xpect_true();

		let event = AnalyticsEvent::new("/about", AnalyticsEventData::Request {
			status: 200,
			method: "GET".into(),
			user_agent: None,
			referrer: None,
		});
		let id = event.id;
		world.trigger(event);
		AsyncRunner::settle_async_tasks(&mut world).await;

		// the event landed in the referenced entity's store
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
}
