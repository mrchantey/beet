use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use serde_json::Value;

/// Create the default router configuration, providing
/// three groups of `children![]` to run in between the
/// default endpoints and fallbacks.
///
///
/// - Waits for all [`Ready`] actions to complete before
///   inserting the server
/// - uses an [`InfallibleSequence`] to ensure
///   all children run.
/// - Runs a [`Fallback`] with common fallback
///   handlers
/// - Inserts an [`assets_bucket`]
/// - Inserts an [`analytics_handler`]
pub fn default_router(
	// runs before default request middleware
	request_middleware: impl Bundle,
	// the actual routes
	endpoints: impl Bundle,
	// runs after `endpoints` and default endpoints
	response_middleware: impl Bundle,
) -> impl Bundle {
	(
		Name::new("Router Root"),
		insert_on_ready(HttpRouter::new()),
		InfallibleSequence,
		children![
			(Name::new("Request Middleware"), request_middleware),
			(Name::new("Endpoints Root"), endpoints),
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
								html_bundle_to_response(),
								assets_bucket(),
								html_bucket(),
								// default not found handled by Router
							]),
						]
					),
				]
			),
			(Name::new("Response Middleware"), response_middleware),
		],
	)
}

/// Create a [`ReadyOnChildrenReady`], allowing any
/// [`ReadyAction`] children to complete before inserting the
/// [`Router`] which will immediately start handling requests.
pub fn insert_on_ready(bundle: impl Send + Clone + Bundle) -> impl Bundle {
	(
		GetReadyOnStartup,
		ReadyOnChildrenReady::default(),
		OnSpawn::observe(move |ev: On<Ready>, mut commands: Commands| {
			if ev.event_target() == ev.original_event_target() {
				commands.entity(ev.event_target()).insert(bundle.clone());
			}
		}),
	)
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
		ReadyAction::new_local(async |entity| {
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
pub fn html_bucket() -> impl Bundle {
	(
		Name::new("Html Bucket"),
		ReadyAction::new_local(async |entity| {
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

	#[sweet::test]
	#[rustfmt::skip]
	async fn works() {
		RouterPlugin::world()
			.spawn((
				super::insert_on_ready(Router),
				EndpointBuilder::get(),
				children![(
					EndWith(Outcome::Pass),
					ReadyAction::new(async |_| {})
				)],
			))
			.await_ready()
			.await
			.oneshot("/")
			.await
			.status()
			.xpect_eq(StatusCode::OK);
	}

	#[sweet::test]
	async fn test_app_info() {
		RouterPlugin::world()
			.with_resource(pkg_config!())
			.spawn((Router, InfallibleSequence, children![
				app_info(),
				html_bundle_to_response()
			]))
			.oneshot_str("/app-info")
			.await
			.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
	}
	#[cfg(feature = "server")]
	#[sweet::test]
	async fn test_default_router() {
		let mut world = RouterPlugin::world();
		world.insert_resource(pkg_config!());
		let mut entity = world.spawn(default_router(
			EndWith(Outcome::Pass),
			// EndWith(Outcome::Pass),
			(Sequence, children![
				EndpointBuilder::get().with_path("foobar"),
			]),
			EndWith(Outcome::Pass),
		));

		entity
			.await_ready()
			.await
			.oneshot_str("/app-info")
			.await
			.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
		entity
			.await_ready()
			.await
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
}
