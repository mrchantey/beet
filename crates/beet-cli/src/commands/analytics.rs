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
	/// The daily aggregate bucket or directory, defaulting to the raw store's
	/// name plus `-rollup`.
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
/// Two blob stores form one report: daily [`AnalyticsRollup`] rows carry the
/// compacted history, while uncompacted segments carry the live tail. A date is
/// read from exactly one source.
///
/// ```sh
/// beet analytics summary                          # local target/stores/analytics
/// beet analytics summary --dir /data/analytics    # a specific directory
/// beet analytics summary --remote --bucket my-site--prod--analytics
/// ```
#[action(route = "analytics/*args", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsParams>())]
pub async fn AnalyticsReport(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();

	// the raw segments and the aggregate rows, both json over blobs: local
	// `FsStore` directories, or the cloud buckets with `--remote` (the same
	// backends a running server derives). The aggregate store is named the way
	// the deploy names it, the raw store plus `-rollup`, which is what
	// `<S3BucketBlock label="analytics-rollup"/>` composes beside `analytics`.
	let rollup_name = |events: &str| {
		parts
			.get_param("rollup")
			.map(String::from)
			.unwrap_or_else(|| format!("{events}-rollup"))
	};
	let (store, rollups) = if parts.has_param("remote") {
		let Some(bucket) = parts.get_param("bucket") else {
			bevybail!(
				"`--remote` requires `--bucket <bucket-name>`, ie `my-app--prod--analytics`"
			);
		};
		(
			AnalyticsStore::remote(bucket)?,
			Table::<AnalyticsRollup>::remote_blob(&rollup_name(bucket))?,
		)
	} else {
		let dir = match parts.get_param("dir") {
			Some(dir) => AbsPathBuf::new(dir)?,
			None => ServiceAccess::local_store_dir("analytics").into_abs(),
		};
		let rollup_dir = AbsPathBuf::new(rollup_name(&dir.to_string()))?;
		(
			AnalyticsStore::local(dir),
			Table::<AnalyticsRollup>::local(rollup_dir),
		)
	};

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

	/// One report over two stores, and a date is read from exactly one of them:
	/// the raw store answers the live tail no aggregate covers yet, the rollup
	/// store answers the compacted history whose segments are gone.
	#[beet::test]
	async fn reads_segment_and_json_over_blob_rollup_stores() {
		let temp = TempDir::new().unwrap();
		let raw_dir = AbsPathBuf::new(temp.path().join("raw")).unwrap();
		let rollup_dir = AbsPathBuf::new(temp.path().join("rollup")).unwrap();
		let raw = AnalyticsStore::local(raw_dir.clone());
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
		let rollups = Table::<AnalyticsRollup>::local(rollup_dir.clone());
		for row in AnalyticsRollup::from_events(&[old]) {
			rollups.push(row).await.unwrap();
		}

		report(&format!(
			"analytics summary --dir {raw_dir} --rollup {rollup_dir}"
		))
		.await
		.as_str()
		.xpect_contains("2 events: 2 page views");
	}
}
