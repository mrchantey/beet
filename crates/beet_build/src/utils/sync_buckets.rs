use beet_core::prelude::*;
use beet_net::prelude::*;
use bevy::tasks::futures_lite;

pub fn push_assets(
	mut commands: Commands,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let src = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	let dst = format!("s3://{}/", pkg_config.assets_bucket_name());
	commands.run_system_cached_with(sync, (src, dst));
	Ok(())
}
pub fn pull_assets(
	mut commands: Commands,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let src = format!("s3://{}/", pkg_config.assets_bucket_name());
	let dst = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	commands.run_system_cached_with(sync, (src, dst));
	Ok(())
}

pub fn push_html(
	mut commands: Commands,
	ws_config: Res<WorkspaceConfig>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let src = ws_config.html_dir.into_abs().to_string();
	let dst = format!("s3://{}", pkg_config.html_bucket_name());
	commands.run_system_cached_with(sync, (src, dst));
	Ok(())
}

fn sync(In((src, dst)): In<(String, String)>) {
	// TODO async systems (beet_flow w/ bevy 0.17)

	// commands.run_system_cached_with(
	// 	AsyncTask::spawn_with_queue_unwrap,
	// 	async move |_queue| {
	futures_lite::future::block_on(async move {
		S3Sync {
			src,
			dst,
			delete: true,
			..default()
		}
		.send()
		.await
	})
	.unwrap();
	// 		Ok(())
	// 	},
	// );
}
