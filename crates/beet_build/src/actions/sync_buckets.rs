use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;
use beet_rsx::prelude::*;

#[construct]
pub fn PushAssets(pkg_config: Res<PackageConfig>) -> Result<impl Bundle> {
	let src = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	let dst = format!("s3://{}/", pkg_config.assets_bucket_name());
	(Name::new("Push Assets"), RunS3Sync { src, dst }).xok()
}

#[construct]
pub fn PullAssets(pkg_config: Res<PackageConfig>) -> Result<impl Bundle> {
	let src = format!("s3://{}/", pkg_config.assets_bucket_name());
	let dst = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	(Name::new("Pull Assets"), RunS3Sync { src, dst }).xok()
}


#[construct]
pub fn PushHtml(
	ws_config: Res<WorkspaceConfig>,
	pkg_config: Res<PackageConfig>,
) -> impl Bundle {
	let src = ws_config.html_dir.into_abs().to_string();
	let dst = format!("s3://{}", pkg_config.html_bucket_name());
	(Name::new("Push Html"), RunS3Sync { src, dst })
}


#[construct]
fn RunS3Sync(src: String, dst: String) {
	OnSpawn::observe(move |mut ev: On<GetOutcome>| {
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
	})
}
