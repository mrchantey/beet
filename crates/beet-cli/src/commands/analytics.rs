use beet::prelude::*;

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
/// Two tables form one report: the daily [`AnalyticsRollup`] rows carry the
/// history, and the raw `analytics_event` rows the days after the last of
/// them. A day is read from exactly one.
///
/// ```sh
/// beet --entry=site analytics summary                          # the site's declared store
/// beet --entry=site --service-access=remote analytics summary  # the same declaration, deployed
/// beet analytics summary --store sqlite:target/stores/analytics.db
/// beet analytics summary --store dynamo://beet-site--prod--analytics
/// ```
#[action(route = "analytics/summary")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn AnalyticsReport(
	/// The analytics store, ie `sqlite:target/stores/analytics.db` or
	/// `dynamo://my-app--prod--analytics`, for a report over a store no entry
	/// declares. Absent, the report reads the store the entry's
	/// `AnalyticsConfig` writes to, local or remote by `--service-access`.
	#[param]
	store: Option<StoreUri>,
	/// Report only the raw events, skipping the aggregates the long history
	/// lives in.
	#[param]
	raw_only: bool,
	cx: ActionContext<Request>,
) -> Result<Response> {
	let world = cx.caller.world().clone();
	// a store never written to (no analytics collected yet) summarizes as
	// empty rather than an error, so the command works on a fresh site
	AnalyticsReport::resolve(&world, store.as_ref())
		.await?
		.summary(raw_only)
		.await?
		.to_string()
		.xmap(Response::ok_text)
		.xok()
}

impl AnalyticsReport {
	/// The analytics tables `store` names, else the one the entry's config
	/// writes to.
	async fn resolve(
		world: &AsyncWorld,
		store: Option<&StoreUri>,
	) -> Result<AnalyticsStore> {
		match store {
			Some(uri) => AnalyticsStore::new(TableStore::from_uri(uri)?).xok(),
			None => {
				let config = one_config(world).await?;
				StoreRef::resolve::<AnalyticsStore>(world, config).await
			}
		}
	}
}

/// Import the analytics history kept as zstd JSONL objects before events were
/// rows, once: every `analytics/raw/**.jsonl.zst` daily archive and unrolled
/// segment in each `--from` store becomes an `analytics_event` row of the
/// analytics store (`--store`, else the entry's), deduplicated, under keys its
/// events determine, so a rerun rewrites the same rows. The old objects are
/// left in place; remove them once the import is checked, then run the rollup
/// with `--full` to rebuild the aggregates.
///
/// ```sh
/// beet --entry=site --stage=prod --service-access=remote analytics import \
/// 	--from s3://beet-site--prod--archive --from s3://beet-site--prod--analytics
/// ```
#[cfg(feature = "legacy_analytics")]
#[action(route = "analytics/import")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn ImportAnalytics(
	/// A store holding legacy `analytics/raw/` objects, repeatable.
	#[param]
	from: Vec<StoreUri>,
	/// The analytics store to import into, absent the one the entry's
	/// `AnalyticsConfig` writes to.
	#[param]
	store: Option<StoreUri>,
	cx: ActionContext<Request>,
) -> Result<Response> {
	if from.is_empty() {
		bevybail!(
			"name the stores holding the legacy objects with `--from <uri>`, ie \
			 `--from s3://my-app--prod--archive`"
		);
	}
	let world = cx.caller.world().clone();
	let sources = from
		.iter()
		.map(BlobStore::from_uri)
		.collect::<Result<Vec<_>>>()?;
	AnalyticsImport::new(
		sources,
		AnalyticsReport::resolve(&world, store.as_ref()).await?,
	)
	.run()
	.await?
	.to_string()
	.xmap(Response::ok_text)
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
			.spawn(FsStore::new(AbsPath::new(temp.path()).unwrap()))
			.id();
		world.spawn((AnalyticsConfig::default(), StoreRef(declared)));
		world
	}

	/// The report and the import mount beside each other, as the site entry
	/// declares them: a history kept as legacy objects imports through the
	/// verb, and the summary then counts it.
	///
	/// REGRESSION: the report took every path under `analytics`, which no
	/// sibling verb can share, so an entry mounting both panicked on boot.
	#[cfg(feature = "legacy_analytics")]
	#[beet::test]
	async fn imports_beside_the_report() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPath::new(temp.path()).unwrap();
		let mut world = crate::commands::render_world();
		let host = world
			.spawn((Router, children![AnalyticsReport, ImportAnalytics]))
			.id();
		let mut run = async |args: String| {
			world
				.entity_mut(host)
				.call::<Request, Response>(
					Request::from_cli_args(CliArgs::parse(&args))
						.with_header::<header::Accept>(vec![MediaType::Text]),
				)
				.await
				.unwrap()
				.unwrap_str()
				.await
		};
		run(format!(
			"analytics import --from=fs:{dir}/old --from=fs:{dir}/older --store=fs:{dir}/new"
		))
		.await
		.xpect_contains("0 events from 0 objects");
		run(format!("analytics summary --store=fs:{dir}/new"))
			.await
			.xpect_contains("0 events");
	}

	/// Summarizing an empty store reports zero events rather than erroring, so
	/// the command works before any analytics have been collected.
	#[beet::test]
	async fn summarizes_empty_store() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPath::new(temp.path()).unwrap();
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

	/// One report over the two tables of one store: the aggregates answer
	/// the rolled up days, the raw events the days after them.
	#[beet::test]
	async fn reads_rollups_and_events() {
		let temp = TempDir::new().unwrap();
		let dir = AbsPath::new(temp.path()).unwrap();
		let store = AnalyticsStore::new(
			TableStore::from_uri(
				&StoreUri::parse(&format!("fs:{dir}")).unwrap(),
			)
			.unwrap(),
		);
		let view = |path: &str, days: u64| {
			AnalyticsEvent::new(path, AnalyticsEventData::PageView {
				duration_ms: 1_000,
				referrer: None,
				title: None,
				client: default(),
			})
			.with_timestamp(
				Timestamp::now() - Duration::from_secs(days * 86_400),
			)
		};
		store.record(view("/recent", 0)).await.unwrap();
		for row in AnalyticsRollup::from_events(&[view("/old", 2)]) {
			store.rollups().put(row).await.unwrap();
		}
		report(&format!("analytics summary --store fs:{dir}"))
			.await
			.as_str()
			.xpect_contains("2 events: 2 page views");
	}
}
