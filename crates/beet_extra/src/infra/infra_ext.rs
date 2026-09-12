//! Shared deploy-example scaffolding, ported from the infra examples' `utils.rs`.
//!
//! Everything here is platform-agnostic so the AWS examples differ only in their
//! deploy target: the block, the build feature set, and the readiness watch. All
//! the AWS infra examples deploy the same content (`examples/bsx_site`) and serve it
//! dynamically from an S3 bucket via the generic `beet` binary. The `#[template]`s in
//! [`templates`](super::templates) wrap these so a `.bsx` deployer composes them.
use beet_core::prelude::*;
use beet_infra::prelude::*;

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

/// The stack's repo store ([`RepoStoreQuery::bootstrap`]) constrained to the
/// http transport; a deploy serving more transports overrides `server`.
pub fn http_bootstrap(
	repos: &RepoStoreQuery,
	entity: Entity,
) -> Result<BootstrapConfig> {
	BootstrapConfig {
		server: Some(RunningSetFilter::new("http")),
		..repos.bootstrap(entity)?
	}
	.xok()
}

/// Bakes `build` into a lambda's [`BuildArtifact`] booting from the stack's
/// repo store ([`http_bootstrap`]), on [`Ready`] once the `{RepoStoreBlock}`
/// declaration has settled: the lambda runtime offers no argv, so the boot
/// config rides the zip's `bootstrap` script.
pub fn lambda_artifact(build: CargoBuild) -> impl Bundle {
	OnSpawn::observe(
		move |ev: On<Ready>,
		      repos: RepoStoreQuery,
		      mut commands: Commands|
		      -> Result {
			commands.entity(ev.entity).insert(
				build
					.clone()
					.with_bootstrap(http_bootstrap(&repos, ev.entity)?)
					.into_lambda_build_artifact()?,
			);
			Ok(())
		},
	)
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
