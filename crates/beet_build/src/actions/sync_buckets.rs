use beet_core::prelude::*;
use beet_flow::prelude::*;
use beet_net::prelude::*;

pub fn push_assets(
	ev: On<GetOutcome>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let src = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	let dst = format!("s3://{}/", pkg_config.assets_bucket_name());
	sync(ev, src, dst);
	Ok(())
}


pub fn pull_assets(
	ev: On<GetOutcome>,
	pkg_config: Res<PackageConfig>,
) -> Result {
	let src = format!("s3://{}/", pkg_config.assets_bucket_name());
	let dst = AbsPathBuf::new_workspace_rel("assets")?.to_string();
	sync(ev, src, dst);
	Ok(())
}

pub fn push_html(
	ev: On<GetOutcome>,
	ws_config: Res<WorkspaceConfig>,
	pkg_config: Res<PackageConfig>,
) {
	let src = ws_config.html_dir.into_abs().to_string();
	let dst = format!("s3://{}", pkg_config.html_bucket_name());
	sync(ev, src, dst);
}

fn sync(mut ev: On<GetOutcome>, src: String, dst: String) {
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
