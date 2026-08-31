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
	/// The remote table/bucket name, used with `--remote`.
	bucket: Option<String>,
	/// The daily aggregate table (or, locally, directory), defaulting to the
	/// events store's name plus `-rollup`.
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
/// Two stores, one report: the daily [`AnalyticsRollup`] aggregates carry the
/// long history (the raws behind them are archived cold and then expired by the
/// table's TTL) and the raw events carry the recent window no aggregate covers
/// yet. A day is read from exactly one of them.
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

	// build the stores: local FsStore directories, or the cloud stores with
	// `--remote` (the same backends a running server derives). The aggregate
	// store is named the way the deploy names it, the events store plus
	// `-rollup`, which is what `<DynamoTableBlock label="analytics-rollup"/>`
	// composes beside `analytics`.
	let rollup_name = |events: &str| {
		parts
			.get_param("rollup")
			.map(String::from)
			.unwrap_or_else(|| format!("{events}-rollup"))
	};
	let (store, rollups) = if parts.has_param("remote") {
		let Some(bucket) = parts.get_param("bucket") else {
			bevybail!(
				"`--remote` requires `--bucket <table-name>`, ie `my-app--prod--analytics`"
			);
		};
		(
			AnalyticsStore::remote(bucket)?,
			Table::<AnalyticsRollup>::remote(&rollup_name(bucket))?,
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
	let events = read_all(&store.store).await?;
	let rollups = match parts.has_param("raw-only") {
		true => Vec::new(),
		false => read_all(&rollups).await?,
	};
	Response::ok_text(AnalyticsSummary::compose(&rollups, &events).to_string())
		.xok()
}

/// Every row of a table that may not exist yet, skipping any it cannot read.
async fn read_all<T: TableStoreRow>(table: &Table<T>) -> Result<Vec<T>> {
	match table.store_exists().await.unwrap_or(false) {
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

	/// Summarizing an empty store reports zero events rather than erroring, so the
	/// command works before any analytics have been collected.
	#[beet::test]
	async fn summarizes_empty_store() {
		let dir =
			AbsPathBuf::new(std::env::temp_dir().join("beet-analytics-empty"))
				.unwrap();
		let mut world = crate::commands::render_world();
		let host = world.spawn((Router, children![AnalyticsReport])).id();
		let response = world
			.entity_mut(host)
			.call::<Request, Response>(
				Request::from_cli_args(CliArgs::parse(&format!(
					"analytics summary --dir {dir}"
				)))
				.with_header::<header::Accept>(vec![MediaType::Text]),
			)
			.await
			.unwrap();
		response.status().is_success().xpect_true();
		response
			.unwrap_str()
			.await
			.as_str()
			.xpect_contains("0 events");
	}
}
