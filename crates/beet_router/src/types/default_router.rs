use crate::prelude::*;
use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use serde_json::Value;



/// Create a [`ReadyOnChildrenReady`], allowing any
/// [`ReadyAction`] children to complete before inserting the
/// [`RouteServer`] which will immediately start handling requests.
pub fn serve_on_ready() -> impl Bundle {
	(
		ReadyOnChildrenReady::default(),
		OnSpawn::observe(|ev: On<Ready>, mut commands: Commands| {
			if ev.event_target() == ev.original_event_target() {
				commands.entity(ev.event_target()).insert(RouteServer);
			}
		}),
	)
}

pub fn analytics_handler() -> impl Bundle {
	ServerAction::new::<_, _, Result, _, _>(
		HttpMethod::Post,
		|In(input): In<Value>, mut commands: Commands| -> Result {
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
			Html(rsx! {
				<main>
					<h1>App Info</h1>
					<p>Title: {title}</p>
					<p>Description: {description}</p>
					<p>Version: {version}</p>
					<p>Stage: {stage}</p>
				</main>
			})
		},
	)
}

pub fn assets_bucket() -> impl Bundle {
	ReadyAction::new_local(async |entity| {
		let (fs_dir, bucket_name, service_access) = entity
			.world()
			.with_then(|world| {
				let fs_dir =
					world.resource::<WorkspaceConfig>().assets_dir.into_abs();
				let bucket_name =
					world.resource::<PackageConfig>().assets_bucket_name();
				let service_access =
					world.resource::<PackageConfig>().service_access;
				(fs_dir, bucket_name, service_access)
			})
			.await;
		let bucket = s3_fs_selector(fs_dir, bucket_name, service_access).await;
		entity
			.insert(
				BucketEndpoint::new(bucket, Some(RoutePath::new("assets")))
					.with_path("assets"),
			)
			.await;
	})
}
/// Bucket for handling html, usually added as a fallback
/// if no request present.
pub fn html_bucket() -> impl Bundle {
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
		let bucket = s3_fs_selector(fs_dir, bucket_name, service_access).await;
		entity.insert(BucketEndpoint::new(bucket, None)).await;
	})
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_flow::prelude::*;
	use beet_net::prelude::*;
	use sweet::prelude::*;

	#[sweet::test]
	#[rustfmt::skip]
	async fn works() {
		RouterPlugin::world()
			.spawn((
				super::serve_on_ready(),
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
			.spawn((RouteServer, InfallibleSequence, children![
				app_info(),
				html_bundle_to_response()
			]))
			.oneshot_str("/app-info")
			.await
			.xpect_contains("<h1>App Info</h1><p>Title: beet_router</p>");
	}
}
