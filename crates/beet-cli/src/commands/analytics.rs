use beet::prelude::*;

/// Request params for the [`AnalyticsReport`] command, surfaced in `--help`.
#[derive(Reflect, Default)]
#[reflect(Default)]
struct AnalyticsParams {
	/// Directory of a local analytics store (default: `target/analytics`).
	dir: Option<String>,
	/// Query the remote (cloud) analytics store instead of a local directory.
	remote: Option<bool>,
	/// The remote table/bucket name, used with `--remote`.
	bucket: Option<String>,
}

/// Summarize collected analytics: what kinds of clients connected, the pages they
/// viewed, and for how long.
///
/// Reads a local analytics directory (an [`FsStore`], the same one a dev server
/// writes) by default, or the live cloud store with `--remote`. The one query
/// surface over both stores.
///
/// ```sh
/// beet analytics summary                          # local target/analytics
/// beet analytics summary --dir /data/analytics    # a specific directory
/// beet analytics summary --remote --bucket my-site--prod--analytics
/// ```
#[action(route = "analytics/*args", handler_only)]
#[derive(Component, Reflect)]
#[reflect(Component)]
#[require(ParamsPartial = ParamsPartial::new::<AnalyticsParams>())]
pub async fn AnalyticsReport(cx: ActionContext<Request>) -> Result<Response> {
	let parts = cx.input.request_parts();

	// build the store: a local FsStore directory, or the cloud store with
	// `--remote` (the same `dynamo_fs_selector` a running server uses).
	let dir = match parts.get_param("dir") {
		Some(dir) => AbsPathBuf::new(dir)?,
		None => WorkspaceConfig::default().analytics_dir.into_abs(),
	};
	let access = if parts.has_param("remote") {
		ServiceAccess::Remote
	} else {
		ServiceAccess::Local
	};
	let bucket = parts.get_param("bucket").unwrap_or("beet--analytics");
	let store = dynamo_fs_selector::<AnalyticsEvent>(
		&dir,
		bucket,
		"us-west-2",
		access,
	)
	.await;

	// a store that was never written to (no analytics collected yet) reads as
	// empty rather than an error, so the command works on a fresh site. The
	// lossy read skips (and warns on) legacy-schema or corrupt rows rather than
	// failing the whole summary.
	let events = if store.store_exists().await.unwrap_or(false) {
		store
			.get_all_lossy()
			.await?
			.into_iter()
			.map(|(_, event)| event)
			.collect::<Vec<_>>()
	} else {
		Vec::new()
	};
	Response::ok_text(AnalyticsSummary::from_events(&events).to_string()).xok()
}

#[cfg(test)]
mod test {
	use super::*;

	/// Summarizing an empty store reports zero events rather than erroring, so the
	/// command works before any analytics have been collected.
	#[beet::test]
	async fn summarizes_empty_store() {
		let dir = AbsPathBuf::new(std::env::temp_dir().join("beet-analytics-empty"))
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
