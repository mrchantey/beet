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

/// The label a stack declares its REPO STORE under, and therefore the one
/// [`remote_bootstrap`] names. Re-exported from [`RepoBucket`] rather than
/// restated, because the two ends of the reference have to be the same string:
/// the deploy publishes into this store and the shipped binary reads out of it.
pub const REPO_BUCKET_LABEL: &str = RepoBucket::LABEL;

/// The one store an app is served from: a per-stage replica of the checkout, so
/// everything the binary reads (the entry, the routes, the assets) is one store.
///
/// Deploy-versioned, by taking the default: every deploy publishes the checkout
/// under its own id and ships a binary that reads that prefix, so the window
/// between the sync and the binary swap is not a window where the old binary
/// parses the new document. See [`RepoBucket`].
///
pub fn repo_bucket() -> S3BucketBlock { S3BucketBlock::new(REPO_BUCKET_LABEL) }

/// The resolved name of `stack`'s repo store, ready to inject so the deployed
/// binary reconstructs the same store. Deterministic for a given stack (identity
/// only, independent of the per-deploy id), so a throwaway stack rebuilt from
/// the same `app_name` resolves the same store.
pub fn repo_bucket_name(stack: &ResolvedStack) -> String {
	stack.resource_name(REPO_BUCKET_LABEL)
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
/// out of `stack`'s repo store, rooted at this deploy's own prefix
/// (`s3://<store>/<deploy id>`, self-rooted so the entry document is probed
/// there) and constrained to the http transport. A deploy serving more
/// transports overrides `server`.
///
/// Takes the resolved stack rather than a store name because the name composes
/// from the stack and from nothing else, so a caller that passed one would only
/// be doing this function's job first.
///
/// Each block renders it at its own platform boundary, splitting boot selection
/// onto argv (the Dockerfile `CMD`, the systemd `ExecStart`, the lambda
/// `bootstrap` script) and service config onto env.
///
/// ## Why the deploy id is baked in rather than resolved at run time
///
/// A deploy publishes the site into the store and THEN swaps the binary that
/// serves it, because the store has to hold the site before the process that
/// reads it exists. At one mutable location that ordering is a window in which
/// the OLD binary reads the NEW document, which is a hard parse failure the
/// moment the document uses syntax that binary predates. Giving each deploy its
/// own prefix closes the window: a binary only ever reads the document it
/// shipped with.
///
/// It also makes a rollback whole for free. Re-applying with an earlier deploy
/// id swaps the process back to that version's artifact, and that artifact was
/// baked pointing at that version's document, so the binary and the document
/// move together with nothing keeping them in step.
///
/// Baked UNCONDITIONALLY rather than threaded from the store's
/// `deploy_versioned` flag: it defaults to true, and the only stores that want
/// it off are runtime data stores, which are never served from. The store that
/// IS served from is held to the invariant by [`RepoBucket`], which every caller
/// of this declares alongside the block.
///
/// A deploy target that cannot bake a per-deploy value into its own boot config
/// does not call this at all. It sets no `repo`, and the release pointer it
/// resolves per start publishes one instead: see [`ArtifactLedger::repo`].
pub fn remote_bootstrap(
	stack: &ResolvedStack,
	deploy_id: &Uuid,
) -> Result<BootstrapConfig> {
	BootstrapConfig {
		// through the store declaration, so an argv a binary bakes and an env a
		// release pointer publishes cannot describe the same store differently
		repo: Some(repo_bucket().store_uri(stack, Some(deploy_id))),
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

/// Sync `examples/bsx_site` (the no-code site) to the app bucket, the content
/// every infra example serves. A mirror, so a renamed or removed source file
/// does not linger in the bucket across deploys.
///
/// Into the launch's own deploy prefix, since [`repo_bucket`] is deploy
/// versioned: that is the prefix the binary this same deploy ships was baked to
/// read. A content-only `sync` verb therefore runs `<AdoptCurrentDeploy/>`
/// first, so the launch adopts the LIVE version's id rather than minting one
/// nothing is serving.
pub fn sync_site(
	stack: &ResolvedStack,
	deployment: &Deployment,
) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("examples/bsx_site")),
			repo_bucket().stack_store(stack, deployment),
		),
		SyncS3Bucket {
			delete: true,
			..default()
		},
	)
}
