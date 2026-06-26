//! Markup wrappers for the infra example types that are not directly spawnable.
//!
//! Most deploy types are reflect components spawned directly by tag (eg
//! `<CloudflareWorkerBlock/>`, `<CloudflareWorkerDeployAction/>`). The wrappers here
//! cover the rest: types that build a non-`Reflect` value (a [`Stack`]'s `MultiMap`, a
//! [`BuildArtifact`]'s `ChildProcess`, an `S3BucketBlock`'s bindings) or compute
//! stack-derived config. A thin `#[template]` wraps each: its props struct is
//! reflect-registered, its body builds the bundle.
use crate::infra::infra_ext;
use beet_core::prelude::*;
use beet_infra::prelude::*;
use beet_net::prelude::*;
use beet_router::prelude::*;

/// `<BeetBinaryBuild features="aws_sdk"/>` — builds the generic `beet` binary
/// (release, zigbuild) with the given feature set as a [`BuildArtifact`], the markup
/// form of the infra examples' `build_beet_binary`. A deploy that ships a binary (a
/// container image, a Lambda zip) reads the produced artifact from its sibling.
#[template]
pub fn BeetBinaryBuild(#[prop(into)] features: String) -> impl Bundle {
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
		.into_build_artifact()
}

/// `<StackHost app_name="hello_lambda">` — the IaC deployer host: a [`Stack`] (so the
/// blocks + verbs resolve it by ancestry), a one-shot [`CliServer`], the default
/// router, [`BootOnLoad`], the standard IaC verb routes (validate/plan/apply/...), and
/// a slot for the example's own deploy/sync/watch routes. The markup form of
/// `(stack(name), stack_cli())`.
#[template]
pub fn StackHost(#[prop(into)] app_name: String) -> impl Bundle {
	(
		infra_ext::stack(app_name),
		CliServer::default(),
		default_router(),
		BootOnLoad,
		children![
			Validate,
			Plan,
			Apply,
			Show,
			List,
			Destroy,
			Rollback,
			Rollforward,
			SlotTarget::new(),
		],
	)
}

/// `<SiteBucket/>` — the S3 bucket the site is served from (non-versioned). Resolves
/// its [`Stack`] by ancestry. The markup form of `site_bucket()`.
#[template]
pub fn SiteBucket() -> impl Bundle { infra_ext::site_bucket() }

/// `<SiteSync app_name="hello_lambda"/>` — publish `examples/bsx_site` to the stack's
/// site bucket. The markup form of `sync_site(stack)`.
#[template]
pub fn SiteSync(#[prop(into)] app_name: String) -> impl Bundle {
	infra_ext::sync_site(&infra_ext::stack(app_name))
}

/// `<LambdaSiteBlock app_name="hello_lambda" features="lambda,aws_sdk"/>` — the lambda
/// deploy block (wired to serve the site from the stack's bucket via `remote_env`) plus
/// its build artifact, on one entity. They share an entity because `TofuApplyAction`
/// pairs the `BuildArtifact` with the block on the same entity to upload it under the
/// block's label, the S3 key the lambda reads its code from. The markup form of the
/// rust example's `(block, build_beet_lambda_binary(features))` deploy child.
#[template]
pub fn LambdaSiteBlock(
	#[prop(into)] app_name: String,
	#[prop(into)] features: String,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	(
		LambdaBlock::default().with_env_vars(infra_ext::remote_env(
			infra_ext::site_bucket_name(&stack),
		)),
		infra_ext::beet_cargo_build(features).into_lambda_build_artifact(),
	)
}

/// `<LambdaWatch app_name="hello_lambda" timeout="30s"/>` — tail the deployed
/// lambda's logs. The markup form of [`AwsWatch::for_lambda`].
#[template]
pub fn LambdaWatch(
	#[prop(into)] app_name: String,
	timeout: Option<Duration>,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	let watch = AwsWatch::for_lambda(&stack, &LambdaBlock::default());
	match timeout {
		Some(timeout) => watch.with_timeout(timeout),
		None => watch,
	}
}

/// `<LightsailSiteBlock app_name="hello_lightsail" features="aws_sdk"/>` — the
/// lightsail deploy block (wired to serve the site from the stack's bucket via
/// `remote_env`) plus its build artifact, on one entity (paired by `TofuApplyAction`,
/// see [`LambdaSiteBlock`]). The markup form of `(block, build_beet_binary(features))`.
#[template]
pub fn LightsailSiteBlock(
	#[prop(into)] app_name: String,
	#[prop(into)] features: String,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	(
		LightsailBlock::default().with_env_vars(infra_ext::remote_env(
			infra_ext::site_bucket_name(&stack),
		)),
		infra_ext::beet_cargo_build(features).into_build_artifact(),
	)
}

/// `<LightsailWatch app_name="hello_lightsail" timeout="30s"/>` — tail the deployed
/// instance's logs. The markup form of [`AwsWatch::for_lightsail`].
#[template]
pub fn LightsailWatch(
	#[prop(into)] app_name: String,
	timeout: Option<Duration>,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	let watch = AwsWatch::for_lightsail(&stack, &LightsailBlock::default());
	match timeout {
		Some(timeout) => watch.with_timeout(timeout),
		None => watch,
	}
}

/// `<FargateSiteBlock app_name="hello_fargate"/>` — the fargate deploy block wired to
/// serve the site from the stack's bucket (`remote_env`). Named to avoid the
/// [`FargateBlock`] it builds.
#[template]
pub fn FargateSiteBlock(#[prop(into)] app_name: String) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	FargateBlock::default()
		.with_env_vars(infra_ext::remote_env(infra_ext::site_bucket_name(&stack)))
}

/// `<FargateWatch app_name="hello_fargate" timeout="300s"/>` — tail the deployed
/// service's logs. The markup form of [`AwsWatch::for_fargate`].
#[template]
pub fn FargateWatch(
	#[prop(into)] app_name: String,
	timeout: Option<Duration>,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	let watch = AwsWatch::for_fargate(&stack, &FargateBlock::default());
	match timeout {
		Some(timeout) => watch.with_timeout(timeout),
		None => watch,
	}
}
