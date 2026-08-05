//! Shared deploy-example scaffolding, ported from the infra examples' `utils.rs`.
//!
//! Everything here is platform-agnostic so the AWS examples differ only in their
//! deploy target: the block, the build feature set, and the readiness watch. All
//! the AWS infra examples deploy the same content (`examples/bsx_site`) and serve it
//! dynamically from an S3 bucket via the generic `beet` binary. The `#[template]`s in
//! [`templates`](super::templates) wrap these so a `.bsx` deployer composes them.
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;

/// Namespaces every cloud resource for an example; `us-west-2` matches the other AWS
/// examples.
pub fn stack(app_name: impl Into<SmolStr>) -> Stack {
	Stack::new(app_name).with_aws_region("us-west-2")
}

/// The single bucket the site is served from. Non-versioned so `sync` overwrites in
/// place and the running binary reads a stable root.
pub fn site_bucket() -> S3BucketBlock {
	S3BucketBlock::new("site").with_deploy_versioned(false)
}

/// The resolved name of the site bucket for this stack, ready to inject so the
/// deployed binary reconstructs the same store. Deterministic for a given stack
/// (the `resource_ident`, independent of the per-deploy id), so a throwaway stack
/// rebuilt from the same `app_name` resolves the same bucket.
pub fn site_bucket_name(stack: &Stack) -> String {
	site_bucket().store(stack).bucket_name().to_string()
}

/// The self-rooted entry-store arg for a deployed site bucket:
/// `--store=s3://<bucket>` (the entry document is probed at the bucket root).
/// Deploy config rides argv, not env; each block converts at its platform
/// boundary (the Dockerfile `CMD`, the systemd `ExecStart`, the lambda
/// `bootstrap` script).
pub fn store_arg(bucket_name: impl AsRef<str>) -> SmolStr {
	format!("--store=s3://{}", bucket_name.as_ref()).into()
}

/// The args the deployed generic `beet` binary is launched with to serve the
/// site from the bucket: [`store_arg`] plus `--server=http`, constraining the
/// boot to the http transport. A deploy serving more transports passes
/// [`store_arg`] and its own `--server` selection instead.
pub fn remote_args(bucket_name: impl AsRef<str>) -> Vec<SmolStr> {
	vec![store_arg(bucket_name), "--server=http".into()]
}

/// Shared `CargoBuild` for the generic `beet` binary; callers pick the terminal
/// (`into_build_artifact` vs `into_lambda_build_artifact`). `--no-default-features`
/// keeps the http-only deploy lean; the mini http backend is always present.
pub fn beet_cargo_build(features: impl Into<SmolStr>) -> CargoBuild {
	CargoBuild::default()
		.with_target(BuildTarget::Zigbuild)
		.with_package("beet-cli")
		.with_binary("beet")
		.with_additional_args(vec![
			"--no-default-features".into(),
			"--features".into(),
			features.into(),
		])
}

/// Sync `examples/bsx_site` (the no-code site) to the bucket root, the content every
/// infra example serves.
pub fn sync_site(stack: &Stack) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("examples/bsx_site")),
			site_bucket().store(stack),
		),
		SyncS3BucketAction,
	)
}
