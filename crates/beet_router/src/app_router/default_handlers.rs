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
pub fn assets_bucket(
	ws_config: When<Res<WorkspaceConfig>>,
	pkg_config: When<Res<PackageConfig>>,
	query: Query<Entity, With<RouterRoot>>,
	mut commands: Commands,
) -> Result {
	let root = query.single()?;
	let fs_dir = ws_config.assets_dir.into_abs();
	let bucket_name = pkg_config.assets_bucket_name();
	commands.spawn((
		ChildOf(root),
		PathFilter::new("assets"),
		AsyncAction::new(async move |mut world, entity| {
			let access = world.resource::<PackageConfig>().service_access;
			let bucket = s3_fs_selector(&fs_dir, &bucket_name, access).await;
			world.entity_mut(entity).insert(bucket);
			world
		}),
		bucket_file_handler(Some(RoutePath::new("assets"))),
	));

	Ok(())
}

pub fn html_bucket(
	ws_config: When<Res<WorkspaceConfig>>,
	pkg_config: When<Res<PackageConfig>>,
	query: Query<Entity, With<RouterRoot>>,
	mut commands: Commands,
) -> Result {
	let root = query.single()?;
	let fs_dir = ws_config.html_dir.into_abs();
	let bucket_name = pkg_config.html_bucket_name();
	commands.spawn((
		ChildOf(root),
		AsyncAction::new(async move |mut world, entity| {
			let access = world.resource::<PackageConfig>().service_access;
			let bucket = s3_fs_selector(&fs_dir, &bucket_name, access).await;
			world.entity_mut(entity).insert(bucket);
			world
		}),
		HandlerConditions::fallback(),
		bucket_file_handler(None),
	));

	Ok(())
}
