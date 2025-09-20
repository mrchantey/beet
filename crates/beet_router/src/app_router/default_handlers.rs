use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;

/// insert the default handlers
pub fn default_handlers(
	mut commands: Commands,
	query: Query<Entity, With<RouterRoot>>,
) -> Result {
	let root = query.single()?;
	commands
		.entity(root)
		.with_child((
			PathFilter::new("/app-info"),
			CacheStrategy::Static,
			HttpMethod::Get,
			HandlerConditions::is_ssr(),
			bundle_endpoint(|config: Res<PackageConfig>| {
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
			}),
		))
		.with_child((
			HandlerConditions::no_response(),
			bundle_to_html_handler(),
		));
	Ok(())
}

pub fn html_bucket(
	ws_config: Res<WorkspaceConfig>,
	pkg_config: Res<PackageConfig>,
	query: Query<Entity, With<RouterRoot>>,
	mut commands: Commands,
) -> Result {
	let root = query.single()?;
	let html_dir = ws_config.html_dir.into_abs();
	#[allow(unused)]
	let html_bucket = pkg_config.html_bucket_name();
	let spawn_bucket = AsyncAction::new(async move |mut world, entity| {
		let access = world.resource::<PackageConfig>().service_access;
		let bucket = match access {
			ServiceAccess::Local => {
				debug!("HTML: Connecting to filesystem: {html_dir}");
				Bucket::new(FsBucketProvider::new(html_dir.clone()), "")
			}
			#[cfg(not(feature = "aws"))]
			ServiceAccess::Remote => {
				warn!(
					"AWS feature not enabled, falling back to local filesystem for HTML bucket."
				);
				Bucket::new(FsBucketProvider::new(html_dir.clone()), "")
			}
			#[cfg(feature = "aws")]
			ServiceAccess::Remote => {
				debug!("HTML: Connecting to S3: {html_bucket}");
				let provider = S3Provider::create().await;
				Bucket::new(provider, html_bucket)
			}
		};
		world.entity_mut(entity).insert(bucket);
		world
	});

	commands.spawn((
		ChildOf(root),
		spawn_bucket,
		HandlerConditions::fallback(),
		bucket_file_handler(),
	));

	Ok(())
}
