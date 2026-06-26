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

/// The env the deployed generic `beet` binary needs to serve the site from the
/// bucket: `BEET_SERVICE_ACCESS=remote` selects the remote store, `BEET_SITE_BUCKET`
/// names it, and `BEET_SERVER=http` constrains the boot to the http transport (a
/// deployed binary is launched with no `--server` arg).
pub fn remote_env(bucket_name: impl Into<SmolStr>) -> Vec<Variable> {
	vec![
		Variable::fixed("BEET_SERVICE_ACCESS", "remote"),
		Variable::fixed("BEET_SITE_BUCKET", bucket_name),
		Variable::fixed("BEET_SERVER", "http"),
	]
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
