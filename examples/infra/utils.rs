//! Shared infra-example scaffolding, included by each `hello_*` example via
//! `#[path = "utils.rs"] mod utils;`.
//!
//! Everything here is platform-agnostic so the examples differ only in their
//! deploy target: the block, the build feature set, and the readiness watch. All
//! five infra examples deploy the same content (`examples/bsx_site`) and serve it
//! dynamically from a bucket via the generic `beet` binary (the same deployable
//! native on Fargate/Lightsail/Lambda, wasm on Workers).
//!
//! Each host is `(stack(name), stack_cli(), deploy_route(..), sync_route(..),
//! watch_route(..))`. `stack_cli()` carries the standard IaC verbs
//! (`validate`/`plan`/`apply`/`destroy`/...) as a `children!` group; the custom
//! routes append via `OnSpawn::insert_child` so they compose with the IaC verbs
//! rather than clobbering them (a second `children!` on the host would).
#![allow(unused, reason = "shared by examples that each use a subset")]
use beet::prelude::*;

/// The standard infra-example entrypoint: load `.env` for the AWS/cloud creds,
/// drop any `AWS_PROFILE` so every subprocess (tofu, the aws CLI, the S3 sync)
/// uses the explicit `.env` keys, then run the deploy app. `infra_scene` builds
/// the platform-specific host spawned on startup.
pub fn deploy_main<B: Bundle>(
	infra_scene: impl 'static + Send + Sync + Fn() -> Result<B>,
) -> AppExit {
	env_ext::load_dotenv();
	unsafe { env_ext::remove_var("AWS_PROFILE") };

	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin {
				level: Level::INFO,
				..default()
			},
			RouterPlugin,
			InfraPlugin,
		))
		.add_systems(Startup, move |mut commands: Commands| -> Result {
			commands.spawn(infra_scene()?).trigger(StartRunning::boot);
			Ok(())
		})
		.run()
}

/// Namespaces every cloud resource for an example; `us-west-2` matches the
/// other AWS examples.
pub fn stack(name: &str) -> Stack {
	Stack::new(name).with_aws_region("us-west-2")
}

/// The single bucket the site is served from. Non-versioned so `sync` overwrites
/// in place and the running binary reads a stable root.
#[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]
pub fn site_bucket() -> S3BucketBlock {
	S3BucketBlock::new("site").with_deploy_versioned(false)
}

/// The env the deployed generic `beet` binary needs to serve the site from the
/// bucket: `BEET_SERVICE_ACCESS=remote` selects the remote store and
/// `BEET_SITE_BUCKET` names it. Injected uniformly through each block's
/// `env_vars` (so it works for Fargate, Lightsail and Lambda alike); the
/// Cloudflare variants extend this list with the R2 endpoint + credentials.
pub fn remote_env(bucket_name: impl Into<SmolStr>) -> Vec<Variable> {
	vec![
		Variable::fixed("BEET_SERVICE_ACCESS", "remote"),
		Variable::fixed("BEET_SITE_BUCKET", bucket_name),
	]
}

/// The resolved name of the site bucket for this stack, ready to inject so the
/// deployed binary reconstructs the same store.
#[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]
pub fn site_bucket_name(stack: &Stack) -> String {
	site_bucket().store(stack).bucket_name().to_string()
}

/// Build the generic `beet` binary (release, zigbuild) with the given feature
/// set. `--no-default-features` keeps the http-only deploy lean (no qrcode / ssh
/// / tui / client transport); the mini http backend is always present.
pub fn build_beet_binary(features: impl Into<SmolStr>) -> impl Bundle {
	beet_cargo_build(features)
		.with_release(true)
		.into_build_artifact()
}

/// Build the generic `beet` binary packaged for Lambda (the `provided.al2023`
/// bootstrap zip rather than a plain executable).
pub fn build_beet_lambda_binary(features: impl Into<SmolStr>) -> impl Bundle {
	beet_cargo_build(features).into_lambda_build_artifact()
}

/// Shared `CargoBuild` for the generic `beet` binary; callers pick the terminal
/// (`into_build_artifact` vs `into_lambda_build_artifact`).
fn beet_cargo_build(features: impl Into<SmolStr>) -> CargoBuild {
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

/// Sync `examples/bsx_site` (the no-code site: `main.bsx`, `routes/`,
/// `templates/`) to the bucket root, the content every infra example serves.
#[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]
pub fn sync_site(stack: &Stack) -> impl Bundle + use<> {
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new("examples/bsx_site")),
			site_bucket().store(stack),
		),
		SyncS3BucketAction,
	)
}

/// The `watch` route: tail the rollout. The platform-specific watch action is
/// supplied by the caller (`AwsWatch::for_*`).
pub fn watch_route(watch: impl Bundle) -> OnSpawn {
	OnSpawn::insert_child(route(
		"watch",
		(exchange_sequence(), children![watch]),
	))
}

/// The `sync` route: re-publish the site to the bucket without rebuilding.
#[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]
pub fn sync_route(stack: &Stack) -> OnSpawn {
	OnSpawn::insert_child(route(
		"sync",
		(exchange_sequence(), children![sync_site(stack)]),
	))
}

/// The non-containerized `deploy` route: declare the block + site bucket, build
/// the binary, apply the infra, sync the site, then watch. Used by the targets
/// that ship a plain binary (Lightsail, Lambda); the containerized targets
/// (Fargate, Cloudflare) inline their own deploy route to slot the image build
/// in after the infra apply.
///
/// The block and its build artifact share one child: `LambdaBlock::apply_to_config`
/// reads the `BuildArtifact` off its own entity for the source hash.
#[cfg(all(feature = "aws_sdk", feature = "bindings_aws_common"))]
pub fn deploy_route(
	block: impl Bundle,
	build: impl Bundle,
	stack: &Stack,
	watch: impl Bundle,
) -> OnSpawn {
	OnSpawn::insert_child(route(
		"deploy",
		(exchange_sequence(), children![
			(block, build),
			site_bucket(),
			TofuApplyAction,
			sync_site(stack),
			watch,
		]),
	))
}
