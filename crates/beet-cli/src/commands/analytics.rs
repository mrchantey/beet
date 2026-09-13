use beet::net::prelude::Table;
use beet::prelude::*;

/// Request params for the [`AnalyticsReport`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsParams {
	/// The analytics store as a store uri, ie
	/// `fs:target/stores/my-app--dev--analytics` or
	/// `s3://my-app--prod--analytics`, for a report over a store no entry
	/// declares. Absent, the report reads the store the entry's
	/// `AnalyticsConfig` writes to, local or remote by `--service-access`.
	store: Option<String>,
	/// A separate store holding the daily aggregates, as a store uri. Absent,
	/// the raw store itself.
	rollup: Option<String>,
	/// Report only the raw events, skipping the aggregates the long history
	/// lives in.
	raw_only: Option<bool>,
}

/// Summarize collected analytics: what kinds of clients connected, the pages they
/// viewed, and for how long.
///
/// Reads exactly the store the entry's [`AnalyticsConfig`] writes to: a
/// document loads whole in every binary, so the serving router's config and
/// its `StoreRef` are in the world when this route dispatches, and the
/// declaration under the `<Stack>` has already resolved the stage and service
/// access of the launch. The one declaration answers writer and reader, and
/// `--service-access=remote` turns the same report onto the cloud store. A
/// store uri names one directly, for a tool with no config in reach.
///
/// Two keyspaces form one report: daily [`AnalyticsRollup`] rows carry the
/// compacted history, while uncompacted segments carry the live tail. A date is
/// read from exactly one source.
///
/// ```sh
/// beet --main=site analytics summary                          # the site's declared store
/// beet --main=site --service-access=remote analytics summary  # the same declaration, deployed
/// beet analytics summary --store fs:target/stores/beet-site--dev--analytics
/// beet analytics summary --store s3://beet-site--prod--analytics
/// ```
#[action(route = "analytics/*args")]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsParams>())]
pub async fn AnalyticsReport(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();
	let world = cx.caller.world().clone();

	// the raw segments and the aggregate rows, both json over blobs and both in
	// ONE store by default: segments own `analytics/raw/`, aggregates own
	// `analytics/rollup/`, so the single `<S3BucketBlock label="analytics"/>`
	// the deploy provisions holds them both. `--rollup` names a store keeping
	// the aggregates apart.
	let store = match parts.get_param("store") {
		Some(uri) => BlobStore::from_uri(&StoreUri::parse(uri)?)?,
		None => {
			let config = one_config(&world).await?;
			StoreRef::resolve::<AnalyticsStore>(&world, config)
				.await?
				.store
		}
	};
	let rollups = match parts.get_param("rollup") {
		Some(uri) => BlobStore::from_uri(&StoreUri::parse(uri)?)?,
		None => store.clone(),
	};
	let store = AnalyticsStore::new(store);
	let rollups = AnalyticsRollup::table(rollups);

	// a store that was never written to (no analytics collected yet) reads as
	// empty rather than an error, so the command works on a fresh site. The
	// lossy read skips (and warns on) legacy-schema or corrupt rows rather than
	// failing the whole summary.
	let events = store.read_all_lossy().await?;
	let rollups = match parts.has_param("raw-only") {
		true => Vec::new(),
		false => read_table(&rollups).await?,
	};
	Response::ok_text(AnalyticsSummary::compose(&rollups, &events).to_string())
		.xok()
}

/// The one [`AnalyticsConfig`] in the world, whose writer the report reads.
/// Several are ambiguous, so the error lists each with the store it has
/// landed, for `--store` to pick from.
async fn one_config(world: &AsyncWorld) -> Result<Entity> {
	let configs = world
		.with_state::<Query<(Entity, Option<&AnalyticsStore>), With<AnalyticsConfig>>, _>(
			|query| {
				query
					.iter()
					.map(|(entity, store)| match store {
						Some(store) => (entity, format!("{entity} ({})", store.describe())),
						None => (entity, entity.to_string()),
					})
					.collect::<Vec<_>>()
			},
		)
		.await;
	match configs.as_slice() {
		[(config, _)] => Ok(*config),
		[] => bevybail!(
			"no analytics store: pass `--store <uri>` (`fs:<dir>`, \
			 `s3://<bucket>`), or load an entry that records analytics, ie \
			 `<Router {{(AnalyticsConfig, StoreRef($analytics))}}>`"
		),
		configs => bevybail!(
			"several `AnalyticsConfig` entities, pass `--store <uri>` to pick \
			 one: {}",
			configs
				.iter()
				.map(|(_, described)| described.as_str())
				.collect::<Vec<_>>()
				.join(", ")
		),
	}
}

/// Reads every valid row from a table whose blob store may not exist yet.
async fn read_table<T: TableStoreRow>(table: &Table<T>) -> Result<Vec<T>> {
	match table.store_exists().await? {
		true => table
			.get_all_lossy()
			.await?
			.into_iter()
			.map(|(_, row)| row)
			.collect(),
		false => Vec::new(),
	}
	.xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// Dispatch the report into `world` with `args`.
	async fn exchange(world: &mut World, args: &str) -> Response {
		let host = world.spawn((Router, children![AnalyticsReport])).id();
		world
			.entity_mut(host)
			.call::<Request, Response>(
				Request::from_cli_args(CliArgs::parse(args))
					.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap()
	}

	/// Run the report in a fresh world, asserting success.
	async fn report(args: &str) -> String {
		let response =
			exchange(&mut crate::commands::render_world(), args).await;
		response.status().is_success().xpect_true();
		response.unwrap_str().await
	}

	/// A world recording analytics to a fresh temp store, as a served entry
	/// spawns it: the config and its `StoreRef` on one entity, the declaration
	/// on another, the report nowhere near either.
	fn recording_world(temp: &TempDir) -> World {
		let mut world = crate::commands::render_world();
		let declared = world
			.spawn(FsStore::new(AbsPathBuf::new(temp.path()).unwrap()))
			.id();
		world.spawn((AnalyticsConfig::default(), StoreRef(declared)));
		world
	}

	/// Summarizing an empty store reports zero events rather than erroring, so
	/// the command works before any analytics have been collected.
	#[beet::test]
	async fn summarizes_empty_store() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPathBuf::new(temp.path()).unwrap();
		report(&format!("analytics summary --store fs:{dir}"))
			.await
			.as_str()
			.xpect_contains("0 events");
	}

	/// With an `AnalyticsConfig` anywhere in the world, the report reads the
	/// writer that config landed, with nothing named on the command line.
	#[beet::test]
	async fn reads_the_configs_store() {
		let temp = TempDir::new().unwrap();
		exchange(&mut recording_world(&temp), "analytics summary")
			.await
			.unwrap_str()
			.await
			.as_str()
			.xpect_contains("0 events");
	}

	/// Two configs are ambiguous: the error lists them and names `--store`.
	#[beet::test]
	async fn several_configs_need_a_store() {
		let temp = TempDir::new().unwrap();
		let mut world = recording_world(&temp);
		let other = world.spawn(InMemoryStore::new()).id();
		world.spawn((AnalyticsConfig::default(), StoreRef(other)));
		exchange(&mut world, "analytics summary")
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("several `AnalyticsConfig`")
			.xpect_contains("--store");
	}

	/// No config and no `--store` is guidance, not a panic.
	#[beet::test]
	async fn no_config_needs_a_store() {
		exchange(&mut crate::commands::render_world(), "analytics summary")
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("no analytics store")
			.xpect_contains("AnalyticsConfig");
	}

	/// One report over the two keyspaces of ONE store, and a date is read from
	/// exactly one of them: `analytics/raw/` answers the live tail no aggregate
	/// covers yet, `analytics/rollup/` answers the compacted history whose
	/// segments are gone.
	#[beet::test]
	async fn reads_segments_and_rollups_from_one_store() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPathBuf::new(temp.path()).unwrap();
		let store = BlobStore::new(FsStore::new(dir.clone()));
		let raw = AnalyticsStore::new(store.clone());
		let recent =
			AnalyticsEvent::new("/recent", AnalyticsEventData::PageView {
				duration_ms: 1_000,
				referrer: None,
				title: None,
				client: default(),
			});
		raw.record(recent).await.unwrap();
		raw.flush().await.unwrap();

		let mut old =
			AnalyticsEvent::new("/old", AnalyticsEventData::PageView {
				duration_ms: 2_000,
				referrer: None,
				title: None,
				client: default(),
			});
		old.timestamp -= 2 * 86_400_000;
		let rollups = AnalyticsRollup::table(store);
		for row in AnalyticsRollup::from_events(&[old]) {
			rollups.push(row).await.unwrap();
		}

		report(&format!("analytics summary --store fs:{dir}"))
			.await
			.as_str()
			.xpect_contains("2 events: 2 page views");
	}
}
