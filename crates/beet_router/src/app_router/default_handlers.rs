use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;
use bevy::prelude::*;

/// insert the default handlers
#[allow(unused_variables)]
pub fn default_handlers(
	mut commands: Commands,
	config: Res<WorkspaceConfig>,
	query: Query<Entity, With<RouterRoot>>,
) -> Result {
	let root = query.single()?;
	let mut root = commands.entity(root);

	root.with_child((
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
	));

	root.with_child((
		HandlerConditions::no_response(),
		bundle_to_html_handler(),
	));

	#[cfg(all(feature = "tokio", not(target_arch = "wasm32")))]
	{
		#[cfg(all(feature = "aws", not(test)))]
		let bucket = s3_bucket();
		#[cfg(not(all(feature = "aws", not(test))))]
		let bucket =
			Bucket::new(FsBucketProvider::new(config.html_dir.into_abs()), "");

		trace!("Inserting default bucket file handler");
		root.with_child((
			bucket,
			HandlerConditions::fallback(),
			bucket_file_handler(),
		));
	}
	Ok(())
}
