use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use serde_json::Value;

// trait BundleFunc:'static+Send+S


/// The entrypoint for a router with two endoints:
/// - `/`: Serve the routes
/// - `/export-static`: export static html
pub fn default_router_cli(spawner: ExchangeSpawner) -> impl Bundle {
	let spawner2 = spawner.clone();
	(
		Name::new("Router CLI"),
		CliServer,
		ExchangeSpawner::new_flow(|| {
			(Fallback, children![
				help_handler(HelpHandlerConfig {
					default_format: HelpFormat::Cli,
					match_root: true,
					introduction: String::from("Router CLI"),
				}),
				EndpointBuilder::new(
					async move |_: (), entity: AsyncEntity| -> Result {
						entity
							.world()
							.insert_resource_then(RenderMode::Ssr)
							.await;
						let html =
							collect_html(entity.world(), &spawner2).await?;
						for (path, html) in html {
							trace!("Exporting html to {}", path);
							fs_ext::write(path, &html)?;
						}
						Ok(())
					}
				)
				.with_path("/export-static"),
				EndpointBuilder::new(
					// dont need to be async but zst restriction for sync systems
					async move |_: (), entity: AsyncEntity| {
						// actually serve the routes
						entity
							.world()
							.spawn_then((
								HttpServer::default(),
								spawner.clone(),
							))
							.await;
						// start serving, never resolve
						std::future::pending::<()>().await;
					}
				)
				.with_path("/")
			])
		}),
	)
}



/// Create the default router configuration, providing
/// three groups of `children![]` to run in between the
/// default endpoints and fallbacks.
///
/// - Waits for all [`Ready`] actions to complete via [`AwaitReady`]
/// - Uses an [`InfallibleSequence`] to ensure all children run
/// - Runs a [`Fallback`] with common fallback handlers
/// - Inserts an [`assets_bucket`]
/// - Inserts an [`analytics_handler`]
pub fn default_router(
	// runs before default request middleware
	request_middleware: impl BundleFunc,
	// the actual routes
	endpoints: impl BundleFunc,
	// runs after `endpoints` and default endpoints
	response_middleware: impl BundleFunc,
) -> ExchangeSpawner {
	ExchangeSpawner::new_flow(move || {
		(InfallibleSequence, children![
			(Name::new("Await Ready"), AwaitReady::default()),
			(
				Name::new("Request Middleware"),
				request_middleware.clone().bundle_func()
			),
			(Name::new("Endpoints Root"), endpoints.clone().bundle_func()),
			(
				Name::new("Default Routes"),
				// Our goal here is to minimize performance overhead
				// of default actions, this pattern ensures default
				// fallbacks only run if no response is present
				Sequence,
				children![
					common_predicates::no_response(),
					(
						Name::new("Default Routes Nested"),
						InfallibleSequence,
						children![
							// # default endpoints
							analytics_handler(),
							app_info(),
							// # default fallbacks
							// stops after first succeeding fallback
							// this is important to avoid response clobbering
							(Name::new("Fallbacks"), Fallback, children![
								help_handler(HelpHandlerConfig {
									default_format: HelpFormat::Http,
									match_root: false,
									introduction: String::new(),
								}),
								html_bundle_to_response(),
								assets_bucket(),
								ssg_html_bucket(),
								// default not found handled by Router
							]),
						]
					),
				]
			),
			(
				Name::new("Response Middleware"),
				response_middleware.clone().bundle_func()
			),
		])
	})
}

pub fn not_found() -> impl Bundle {
	(Name::new("Not Found"), Sequence, children![
		common_predicates::no_response(),
		EndpointBuilder::new(StatusCode::NOT_FOUND).with_trailing_path()
	])
}

pub fn analytics_handler() -> impl Bundle {
	ServerAction::new::<_, _, Result<(), BevyError>, _, _>(
		HttpMethod::Post,
		|In(input): In<Value>,
		 mut commands: Commands|
		 -> Result<(), BevyError> {
			let ev = AnalyticsEvent::parse(input)?;
			commands.trigger(ev);
			Ok(())
		},
	)
	.with_path("/analytics")
}


pub fn app_info() -> EndpointBuilder {
	EndpointBuilder::get().with_path("/app-info").with_handler(
		|config: Res<PackageConfig>| {
			let PackageConfig {
				title,
				description,
				version,
				stage,
				..
			} = config.clone();
			rsx! {
				<main>
					<h1>App Info</h1>
					<p>Title: {title}</p>
					<p>Description: {description}</p>
					<p>Version: {version}</p>
					<p>Stage: {stage}</p>
				</main>
			}
		},
	)
}

pub fn assets_bucket() -> impl Bundle {
	(
		Name::new("Assets Bucket"),
		ReadyAction::run_local(async |entity| {
			let (fs_dir, bucket_name, service_access) = entity
				.world()
				.with_then(|world| {
					let fs_dir = world
						.resource::<WorkspaceConfig>()
						.assets_dir
						.into_abs();
					let bucket_name =
						world.resource::<PackageConfig>().assets_bucket_name();
					let service_access =
						world.resource::<PackageConfig>().service_access;
					(fs_dir, bucket_name, service_access)
				})
				.await;
			let bucket =
				s3_fs_selector(fs_dir, bucket_name, service_access).await;
			entity
				.insert_then(
					BucketEndpoint::new(bucket, Some(RoutePath::new("assets")))
						.with_path("assets"),
				)
				.await;
		}),
	)
}
/// Bucket for handling html, usually added as a fallback
/// if no request present.
/// This only runs in [`RenderMode::Ssg`] to avoid EndpointTree
/// conflicts with the SSR endpoints.
pub fn ssg_html_bucket() -> impl Bundle {
	(
		Name::new("Html Bucket"),
		ReadyAction::run_local(async |entity| {
			if entity
				.world()
				.with_then(|world| {
					world
						.get_resource::<RenderMode>()
						.map(|mode| matches!(mode, RenderMode::Ssr))
						.unwrap_or(false)
				})
				.await
			{
				return;
			}

			let (fs_dir, bucket_name, service_access) = entity
				.world()
				.with_then(|world| {
					let fs_dir =
						world.resource::<WorkspaceConfig>().html_dir.into_abs();
					let bucket_name =
						world.resource::<PackageConfig>().html_bucket_name();
					let service_access =
						world.resource::<PackageConfig>().service_access;
					(fs_dir, bucket_name, service_access)
				})
				.await;
			let bucket =
				s3_fs_selector(fs_dir, bucket_name, service_access).await;
			entity.insert_then(BucketEndpoint::new(bucket, None)).await;
		}),
	)
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;

	#[cfg(feature = "server")]
	#[sweet::test(timeout_ms = 10000)]
	async fn test_default_router() {
		let mut world = RouterPlugin::world();
		world.insert_resource(pkg_config!());
		let mut entity = world.spawn(default_router(
			|| EndWith(Outcome::Pass),
			|| {
				(Sequence, children![
					EndpointBuilder::get().with_path("foobar"),
				])
			},
			|| EndWith(Outcome::Pass),
		));

		entity
			.oneshot_str("/app-info")
			.await
			.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
		entity
			.oneshot("/assets/branding/logo.png")
			.await
			.into_result()
			.await
			.unwrap();
		let mut stat = async |val: &str| entity.oneshot(val).await.status();
		stat("/bingbong").await.xpect_eq(StatusCode::NOT_FOUND);
		stat("/assets/bing").await.xpect_eq(StatusCode::NOT_FOUND);
		stat("/assets/branding/logo.png")
			.await
			.xpect_eq(StatusCode::OK);
		stat("/foobar").await.xpect_eq(StatusCode::OK);
	}

	#[sweet::test(timeout_ms = 5000)]
	async fn test_app_info() {
		RouterPlugin::world()
			.with_resource(pkg_config!())
			.spawn(ExchangeSpawner::new_flow(|| {
				(InfallibleSequence, children![
					app_info(),
					html_bundle_to_response()
				])
			}))
			.oneshot_str("/app-info")
			.await
			.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
	}
}
