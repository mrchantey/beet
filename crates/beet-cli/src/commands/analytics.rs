use beet::net::prelude::Table;
use beet::prelude::*;

/// Request params for the [`AnalyticsReport`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsParams {
	/// Directory of a local analytics store (default: `target/stores/analytics`).
	dir: Option<String>,
	/// Query the remote (cloud) analytics store instead of a local directory.
	remote: Option<bool>,
	/// The remote raw-segment bucket name, used with `--remote`.
	bucket: Option<String>,
	/// A separate bucket or directory holding the daily aggregates, which
	/// otherwise share the raw store.
	rollup: Option<String>,
	/// Report only the raw events, skipping the aggregates the long history
	/// lives in.
	raw_only: Option<bool>,
}

/// Summarize collected analytics: what kinds of clients connected, the pages they
/// viewed, and for how long.
///
/// Reads a local analytics directory (an [`FsStore`], the same one a dev server
/// writes) by default, or the live cloud store with `--remote`. The one query
/// surface over both stores.
///
/// Two keyspaces form one report: daily [`AnalyticsRollup`] rows carry the
/// compacted history, while uncompacted segments carry the live tail. A date is
/// read from exactly one source.
///
/// ```sh
/// beet analytics summary                          # local target/stores/analytics
/// beet analytics summary --dir /data/analytics    # a specific directory
/// beet analytics summary --remote --bucket my-site--prod--analytics
/// ```
#[action(route = "analytics/*args")]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsParams>())]
pub async fn AnalyticsReport(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();

	// the raw segments and the aggregate rows, both json over blobs and both in
	// ONE store by default: segments own `analytics/raw/`, aggregates own
	// `analytics/rollup/`, so the single `<S3BucketBlock label="analytics"/>`
	// the deploy provisions holds them both. `--rollup` names a store keeping
	// the aggregates apart, for a deployment whose refs point at two
	// declarations.
	let (store, rollups) = if parts.has_param("remote") {
		let Some(bucket) = parts.get_param("bucket") else {
			bevybail!(
				"`--remote` requires `--bucket <bucket-name>`, ie `my-app--prod--analytics`"
			);
		};
		(
			AnalyticsStore::remote(bucket)?,
			BlobStore::remote(parts.get_param("rollup").unwrap_or(bucket))?,
		)
	} else {
		let dir = match parts.get_param("dir") {
			Some(dir) => AbsPathBuf::new(dir)?,
			None => ServiceAccess::local_store_dir("analytics").into_abs(),
		};
		let rollup_dir = match parts.get_param("rollup") {
			Some(rollup) => AbsPathBuf::new(rollup)?,
			None => dir.clone(),
		};
		(
			AnalyticsStore::local(dir),
			BlobStore::new(FsStore::new(rollup_dir)),
		)
	};
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

	async fn report(args: &str) -> String {
		let mut world = crate::commands::render_world();
		let host = world.spawn((Router, children![AnalyticsReport])).id();
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
		report(&format!("analytics summary --dir {dir}"))
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
		let raw = AnalyticsStore::local(dir.clone());
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
		let rollups =
			AnalyticsRollup::table(BlobStore::new(FsStore::new(dir.clone())));
		for row in AnalyticsRollup::from_events(&[old]) {
			rollups.push(row).await.unwrap();
		}

		report(&format!("analytics summary --dir {dir}"))
			.await
			.as_str()
			.xpect_contains("2 events: 2 page views");
	}
}
