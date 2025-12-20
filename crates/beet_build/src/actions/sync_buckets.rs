use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

#[construct]
pub fn PushAssets() -> impl Bundle {
	(
		Name::new("Push Assets"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>|
			      -> Result {
				let src = AbsPathBuf::new_workspace_rel("assets")?.to_string();
				let dst = format!("s3://{}/", pkg_config.assets_bucket_name());
				s3_sync(ev, src, dst);
				Ok(())
			},
		),
	)
}

#[construct]
pub fn PullAssets() -> impl Bundle {
	(
		Name::new("Pull Assets"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>|
			      -> Result {
				let src = format!("s3://{}/", pkg_config.assets_bucket_name());
				let dst = AbsPathBuf::new_workspace_rel("assets")?.to_string();
				s3_sync(ev, src, dst);
				Ok(())
			},
		),
	)
}


#[construct]
pub fn PushHtml() -> impl Bundle {
	(
		Name::new("Push Html"),
		OnSpawn::observe(
			move |ev: On<GetOutcome>,
			      pkg_config: Res<PackageConfig>,
			      ws_config: Res<WorkspaceConfig>| {
				let src = ws_config.html_dir.into_abs().to_string();
				let dst = format!("s3://{}", pkg_config.html_bucket_name());
				s3_sync(ev, src, dst)
			},
		),
	)
}


fn s3_sync(mut ev: On<GetOutcome>, src: String, dst: String) {
	let src = src.clone();
	let dst = dst.clone();
	ev.run_async(async move |mut action| -> Result {
		S3Sync {
			src,
			dst,
			delete: true,
			..default()
		}
		.send()
		// fatal, propagate error instead of Outcome::Fail
		.await?;
		action.trigger_with_cx(Outcome::Pass);
		Ok(())
	});
}
