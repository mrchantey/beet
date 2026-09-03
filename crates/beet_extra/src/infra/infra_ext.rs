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

/// The one bucket an app is served from: a per-stage replica of the checkout, so
/// everything the binary reads (the entry, the routes, the assets) is one store.
/// Non-versioned so `sync` overwrites in place and the running binary reads a
/// stable root.
pub fn app_bucket() -> S3BucketBlock {
	S3BucketBlock::new("app").with_deploy_versioned(false)
}

/// The resolved name of the app bucket for this stack, ready to inject so the
/// deployed binary reconstructs the same store. Deterministic for a given stack
/// (identity only, independent of the per-deploy id), so a throwaway stack
/// rebuilt from the same `app_name` resolves the same bucket.
pub fn app_bucket_name(stack: &ResolvedStack) -> String {
	stack.resource_name("app")
}

/// A CloudWatch tail of `target`, with an optional timeout after which the
/// follow is killed. The log group composes from the ancestor [`Stack`] when the
/// tail runs, so a watch verb never restates the app identity.
pub fn watch(target: WatchTarget, timeout: Option<Duration>) -> AwsWatch {
	let watch = AwsWatch::for_target(target);
	match timeout {
		Some(timeout) => watch.with_timeout(timeout),
		None => watch,
	}
}

/// The deployed generic `beet` binary's [`BootstrapConfig`] for serving the site
/// from its bucket: the self-rooted `s3://<bucket>` repo store (the entry
/// document is probed at the bucket root) constrained to the http transport. A
/// deploy serving more transports overrides `server`.
///
/// Each block renders it at its own platform boundary, splitting boot selection
/// onto argv (the Dockerfile `CMD`, the systemd `ExecStart`, the lambda
/// `bootstrap` script) and service config onto env.
pub fn remote_bootstrap(
	bucket_name: impl AsRef<str>,
) -> Result<BootstrapConfig> {
	BootstrapConfig {
		repo: Some(StoreUri::parse(&format!("s3://{}", bucket_name.as_ref()))?),
		server: Some(RunningSetFilter::new("http")),
		..default()
	}
	.xok()
}

/// Shared `CargoBuild` for the generic `beet` binary (release, zigbuild);
/// callers pick the terminal (`into_build_artifact` vs
/// `into_lambda_build_artifact`). `--no-default-features` keeps the http-only
/// deploy lean; the mini http backend is always present.
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
		.with_release(true)
}

/// Sync `examples/bsx_site` (the no-code site) to the bucket root, the content every
/// infra example serves. A mirror, so a renamed or removed source file does not
/// linger in the bucket across deploys.
pub fn sync_site(
	stack: &ResolvedStack,
	deployment: &Deployment,
) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("examples/bsx_site")),
			app_bucket().stack_store(stack, deployment),
		),
		SyncS3Bucket::default().with_delete(true),
	)
}
