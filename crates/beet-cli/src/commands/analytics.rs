use beet::net::prelude::Table;
use beet::prelude::*;

/// Request params for the [`AnalyticsReport`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsParams {
	/// The analytics store as a store uri, ie
	/// `fs:target/stores/my-app--dev--analytics` or
	/// `s3://my-app--prod--analytics`, for a report over a store no declaration
	/// binds. Absent, the report reads the declaration its `StoreRef` names,
	/// local or remote by `--service-access`.
	store: Option<String>,
	/// A separate store holding the daily aggregates, as a store uri. Absent,
	/// the `RollupStoreRef` declaration, else the raw store itself.
	rollup: Option<String>,
	/// Report only the raw events, skipping the aggregates the long history
	/// lives in.
	raw_only: Option<bool>,
}

/// Summarize collected analytics: what kinds of clients connected, the pages they
/// viewed, and for how long.
///
/// Reads the stores its `StoreRef` / `RollupStoreRef` relations bind, exactly
/// as the middleware that writes them and the job that compacts them do, so
/// the one declaration answers every reader and `--service-access=remote`
/// turns the same report onto the cloud store. A store uri names one directly,
/// for a tool with no declaration in reach.
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
	// the deploy provisions holds them both. `--rollup` or a `RollupStoreRef`
	// names a store keeping the aggregates apart.
	let store = match parts.get_param("store") {
		Some(uri) => BlobStore::from_uri(&StoreUri::parse(uri)?)?,
		None => match cx
			.caller
			.get::<StoreRef, _>(|store_ref| store_ref.store())
			.await
		{
			Ok(target) => {
				StoreRef::resolve::<BlobStore>(&world, target).await?
			}
			Err(_) => bevybail!(
				"no analytics store: pass `--store <uri>` (`fs:<dir>`, \
				 `s3://<bucket>`), or mount the report beside its declaration, \
				 ie `<AnalyticsReport {{StoreRef($analytics)}}/>`"
			),
		},
	};
	let rollups = match parts.get_param("rollup") {
		Some(uri) => BlobStore::from_uri(&StoreUri::parse(uri)?)?,
		None => match cx
			.caller
			.get::<RollupStoreRef, _>(|store_ref| store_ref.0)
			.await
		{
			Ok(target) => {
				StoreRef::resolve::<BlobStore>(&world, target).await?
			}
			Err(_) => store.clone(),
		},
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

	/// Run the report mounted with `extra` beside it, asserting success.
	async fn report(extra: impl Bundle, args: &str) -> String {
		let mut world = crate::commands::render_world();
		let host = world
			.spawn((Router, children![(AnalyticsReport, extra)]))
			.id();
		let response = world
			.entity_mut(host)
			.call::<Request, Response>(
				Request::from_cli_args(CliArgs::parse(args))
					.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap();
		response.status().is_success().xpect_true();
		response.unwrap_str().await
	}

	/// Summarizing an empty store reports zero events rather than erroring, so
	/// the command works before any analytics have been collected.
	#[beet::test]
	async fn summarizes_empty_store() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPathBuf::new(temp.path()).unwrap();
		report((), &format!("analytics summary --store fs:{dir}"))
			.await
			.as_str()
			.xpect_contains("0 events");
	}

	/// Mounted beside a declaration, the report reads the store the
	/// declaration attached, with nothing named on the command line.
	#[beet::test]
	async fn reads_the_declared_store() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPathBuf::new(temp.path()).unwrap();
		let mut world = crate::commands::render_world();
		let declared = world.spawn(FsStore::new(dir)).id();
		let host = world
			.spawn((Router, children![(AnalyticsReport, StoreRef(declared))]))
			.id();
		world
			.entity_mut(host)
			.call::<Request, Response>(
				Request::from_cli_args(CliArgs::parse("analytics summary"))
					.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap()
			.unwrap_str()
			.await
			.as_str()
			.xpect_contains("0 events");
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

		report((), &format!("analytics summary --store fs:{dir}"))
			.await
			.as_str()
			.xpect_contains("2 events: 2 page views");
	}
}
