//! The [`AnalyticsStore`] backend and the config-triggered bootstrap +
//! persistence observers.
use crate::prelude::*;
use beet_core::prelude::*;

// TODO this should be from beet_infra
const DEFAULT_REGION: &str = "us-west-2";

/// Component holding the analytics storage table, spawned on the
/// [`AnalyticsConfig`] entity.
#[derive(Clone, Deref, DerefMut, Component)]
pub struct AnalyticsStore {
	/// The underlying table store for analytics events.
	pub store: TableStore<AnalyticsEvent>,
}

/// Observer: on an [`AnalyticsConfig`] insertion, create the [`AnalyticsStore`]
/// and (under `geoip`) the country database, both on the config's own entity.
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
		let entity = world.entity(entity);
		// guard against a racing second insertion creating it first.
		if entity.get::<AnalyticsStore, _>(|_| ()).await.is_ok() {
			return Ok(());
		}
		// read the backend config now (the scene has settled), defaulting when a
		// PackageConfig/WorkspaceConfig was not inserted.
		let (fs_dir, table_name) = world
			.with(|world: &mut World| {
				let ws = world
					.get_resource::<WorkspaceConfig>()
					.cloned()
					.unwrap_or_default();
				let pkg = world
					.get_resource::<PackageConfig>()
					.cloned()
					.unwrap_or_default();
				// the remote table name is the deploy-provided `--analytics-table`
				// / `BEET_ANALYTICS_TABLE`, so the deploy owns the name; the
				// package-derived name is the fallback for a self-named build.
				let table_name = BootstrapConfig::get()
					.analytics_table()
					.as_deref()
					.map(str::to_string)
					.unwrap_or_else(|| pkg.analytics_bucket_name());
				(ws.analytics_dir.into_abs(), table_name)
			})
			.await;
		let access = BootstrapConfig::get().service_access();
		let store =
			TableStore::dynamo_fs_selector(&fs_dir, &table_name, DEFAULT_REGION, access)
				.await;
		// the offline country database rides the app's own store (the checkout in
		// dev, the app bucket when deployed), resolved like any other tree
		// consumer. Best-effort: no store, or no blob, just disables lookups.
		let blobs = entity
			.with_state::<AncestorQuery<&BlobStore>, _>(|entity, stores| {
				stores.get(entity).cloned().ok()
			})
			.await?;
		let geoip = match blobs {
			Some(blobs) => GeoIp::load(&blobs).await,
			None => GeoIp::default(),
		};
		entity.insert((AnalyticsStore { store }, geoip)).await?;
		Ok(())
	});
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
		let store = TableStore::<AnalyticsEvent>::temp();
		let event = AnalyticsEvent::new("/about", AnalyticsEventData::Request {
			status: 200,
			method: "GET".into(),
			user_agent: None,
		})
		.with_client_kind(ClientKind::Web);
		let id = event.id;
		store.push(event).await.unwrap();
		let loaded = store.get(id).await.unwrap();
		loaded.path.as_str().xpect_eq("/about");
		loaded.event_kind.xpect_eq(AnalyticsEventKind::Request);
	}

	/// The config's own entity carries the store and the geoip component, so
	/// consumers resolve them by ancestry rather than as globals.
	#[beet_core::test]
	async fn config_entity_carries_the_store() {
		let mut world = (AsyncPlugin, analytics_plugin).into_world();
		let entity = world.spawn(AnalyticsConfig::default()).flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		world.entity(entity).contains::<AnalyticsStore>().xpect_true();
		world.entity(entity).contains::<GeoIp>().xpect_true();
	}
}
