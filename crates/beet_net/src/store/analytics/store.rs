//! The [`AnalyticsStore`] backend and the config-triggered bootstrap +
//! persistence observers.
use crate::prelude::*;
use beet_core::prelude::*;

// TODO this should be from beet_infra
const DEFAULT_REGION: &str = "us-west-2";

/// Resource holding the analytics storage table.
#[derive(Clone, Deref, DerefMut, Resource)]
pub struct AnalyticsStore {
	/// The underlying table store for analytics events.
	pub store: TableStore<AnalyticsEvent>,
}

/// Observer: on the first [`AnalyticsConfig`] insertion, create the
/// [`AnalyticsStore`] and (under `geoip`) load the country database.
///
/// Config-triggered rather than a startup system so it is inert until analytics
/// is switched on, and so it works whenever the config lands (markup scenes
/// resolve asynchronously). Reads the backend config inside the async task, so a
/// [`PackageConfig`] spawned alongside the [`AnalyticsConfig`] is already
/// present; idempotent, so a scene reload does not rebuild the store.
pub(super) fn spawn_store_on_config(
	_ev: On<Add, AnalyticsConfig>,
	existing: Option<Res<AnalyticsStore>>,
	commands: AsyncCommands,
) {
	if existing.is_some() {
		return;
	}
	commands.run(async move |world| {
		// guard against a racing second config insertion creating it first.
		if world
			.with(|world: &mut World| {
				world.get_resource::<AnalyticsStore>().is_some()
			})
			.await
		{
			return Ok(());
		}
		// read the backend config now (the scene has settled), defaulting when a
		// PackageConfig/WorkspaceConfig was not inserted.
		let (fs_dir, assets_dir, bucket_name, access) = world
			.with(|world: &mut World| {
				let ws = world
					.get_resource::<WorkspaceConfig>()
					.cloned()
					.unwrap_or_default();
				let pkg = world
					.get_resource::<PackageConfig>()
					.cloned()
					.unwrap_or_default();
				// the remote table name is the deploy-provided `BEET_ANALYTICS_TABLE`
				// (like `BEET_SITE_BUCKET`), so the deploy owns the name; the
				// package-derived name is the fallback for a self-named build.
				let table_name = env_ext::var("BEET_ANALYTICS_TABLE")
					.unwrap_or_else(|_| pkg.analytics_bucket_name());
				(
					ws.analytics_dir.into_abs(),
					ws.assets_dir.into_abs(),
					table_name,
					pkg.service_access,
				)
			})
			.await;
		let store =
			dynamo_fs_selector(&fs_dir, &bucket_name, DEFAULT_REGION, access)
				.await;
		world.insert_resource(AnalyticsStore { store }).await;
		// the offline country database is a best-effort static asset: a missing
		// or unreadable db just leaves country lookups returning `None`.
		let geoip = GeoIp::load(&assets_store(&assets_dir, access)).await;
		world.insert_resource(geoip).await;
		Ok(())
	});
}

/// The assets [`BlobStore`]: the deploy-provided `BEET_ASSETS_BUCKET` when
/// running remote (the container has no local assets, see `AssetsStore` in
/// beet_router), else the local assets dir.
#[allow(unused_variables)]
fn assets_store(assets_dir: &AbsPathBuf, access: ServiceAccess) -> BlobStore {
	#[cfg(all(feature = "aws_sdk", not(target_arch = "wasm32")))]
	if access == ServiceAccess::Remote {
		if let Ok(bucket) = env_ext::var("BEET_ASSETS_BUCKET") {
			let region = env_ext::var("AWS_REGION")
				.unwrap_or_else(|_| DEFAULT_REGION.to_string());
			return BlobStore::new(S3Store::new(bucket, region));
		}
	}
	BlobStore::new(FsStore::new(assets_dir.clone()))
}

/// Observer: persist a triggered [`AnalyticsEvent`] to the [`AnalyticsStore`].
///
/// The single sink for every emitter (request middleware, navigator, web
/// beacon). A missing store (analytics not switched on) drops the event rather
/// than panicking, since emitters trigger unconditionally. Fire-and-forget: the
/// push runs on the async queue so recording never blocks a request or a
/// navigation.
pub(super) fn handle_analytics_event(
	ev: On<AnalyticsEvent>,
	store: Option<Res<AnalyticsStore>>,
	commands: AsyncCommands,
) {
	let Some(store) = store else {
		return;
	};
	let store = store.clone();
	let event = ev.event().clone();
	commands.run(async move |_| {
		store.push(event).await?;
		Ok(())
	});
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	#[beet_core::test]
	async fn event_roundtrips_through_store() {
		let store = temp_table::<AnalyticsEvent>();
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
}
