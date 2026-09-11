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
/// out of `repo`, the stack's repo store rooted at this deploy's own prefix
/// (`s3://<store>/<deploy id>`, self-rooted so the entry document is probed
/// there, see [`RepoStoreDecl::store_uri`]) and constrained to the http
/// transport. A deploy serving more transports overrides `server`.
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
/// The prefix, the sync's destination and the ledger's record all derive from
/// the store's one erased declaration, so they agree by construction; a store
/// declaring `deploy_versioned=false` is read at its root by every one of them.
///
/// A deploy target that cannot bake a per-deploy value into its own boot config
/// does not call this at all. It sets no `repo`, and the release pointer it
/// resolves per start publishes one instead: see [`ArtifactLedger::repo`].
pub fn remote_bootstrap(repo: StoreUri) -> BootstrapConfig {
	BootstrapConfig {
		repo: Some(repo),
		server: Some(RunningSetFilter::new("http")),
		..default()
	}
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

/// Sync `examples/bsx_site` (the no-code site) to the stack's repo store, the
/// content every infra example serves: a `<DirSync>` of that directory into
/// `repo`, mirrored so a renamed or removed source file does not linger across
/// deploys.
///
/// The per-deploy prefix is the sync's to resolve when it runs, from the store's
/// own declaration: a content-only `sync` verb therefore runs
/// `<AdoptCurrentDeploy/>` first, so the launch adopts the LIVE version's id
/// rather than minting one nothing is serving.
pub fn sync_site(repo: &RepoStoreDecl) -> impl Bundle {
	(
		DirSync::new(repo.label().clone(), "examples/bsx_site"),
		SyncS3Bucket {
			delete: true,
			..default()
		},
	)
}
