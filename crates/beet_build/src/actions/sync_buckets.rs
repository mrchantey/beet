use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

/// Pushes local assets to the configured S3 assets bucket.
#[construct]
pub fn PushAssets() -> impl Bundle {
	(
		Name::new("Push Assets"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>,
			      commands: AsyncCommands|
			      -> Result {
				let src = AbsPathBuf::new_workspace_rel("assets")?.to_string();
				let dst = format!("s3://{}/", pkg_config.assets_bucket_name());
				s3_sync(ev, src, dst, commands);
				Ok(())
			},
		),
	)
}

/// Pulls assets from the configured S3 assets bucket to the local assets directory.
#[construct]
pub fn PullAssets() -> impl Bundle {
	(
		Name::new("Pull Assets"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>,
			      commands: AsyncCommands|
			      -> Result {
				let src = format!("s3://{}/", pkg_config.assets_bucket_name());
				let dst = AbsPathBuf::new_workspace_rel("assets")?.to_string();
				s3_sync(ev, src, dst, commands);
				Ok(())
			},
		),
	)
}


/// Pushes generated HTML files to the configured S3 HTML bucket.
#[construct]
pub fn PushHtml() -> impl Bundle {
	(
		Name::new("Push Html"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>,
			      ws_config: Res<WorkspaceConfig>,
			      commands: AsyncCommands| {
				let src = ws_config.html_dir.into_abs().to_string();
				let dst = format!("s3://{}", pkg_config.html_bucket_name());
				s3_sync(ev, src, dst, commands)
			},
		),
	)
}


fn s3_sync(
	ev: On<GetOutcome>,
	src: String,
	dst: String,
	mut commands: AsyncCommands,
) {
	let src = src.clone();
	let dst = dst.clone();
	let target = ev.target();
	commands.run(async move |world: AsyncWorld| {
		let result = S3Sync {
			src,
			dst,
			delete: true,
			..default()
		}
		.send()
		.await;

		world
			.with_then(move |world| {
				let outcome = if result.is_ok() {
					Outcome::Pass
				} else {
					Outcome::Fail
				};
				world.entity_mut(target).trigger_target(outcome);
			})
			.await;
	});
}
