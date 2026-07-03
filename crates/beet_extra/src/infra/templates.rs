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

/// `<ExampleBinaryBuild example="ssh_tui_site" features="ssh_tui,http_server,markdown"/>`
/// — builds a specific *example* binary (release, zigbuild) as a [`BuildArtifact`], the
/// example-target counterpart of [`BeetBinaryBuild`]. No `--no-default-features` (the
/// example needs the workspace example feature set).
#[template]
pub fn ExampleBinaryBuild(
	#[prop(into)] example: String,
	#[prop(into)] features: String,
) -> impl Bundle {
	CargoBuild::default()
		.with_target(BuildTarget::Zigbuild)
		.with_example(example)
		.with_additional_args(vec!["--features".into(), features.into()])
		.with_release(true)
		.into_build_artifact()
}

/// `<StackHost app_name="lambda">` — the IaC deployer host: a [`Stack`] (so the
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

/// `<BucketStack app_name="bucket-example"/>` — like [`StackHost`] but selects an S3
/// state backend when `--s3-backend` is passed (else local). The markup form of
/// lifecycle.rs's backend toggle.
#[template]
pub fn BucketStack(#[prop(into)] app_name: String) -> impl Bundle {
	let backend: StackBackend =
		if CliArgs::parse_env().params.contains_key("s3-backend") {
			S3Backend::default().into()
		} else {
			LocalBackend::default().into()
		};
	(
		infra_ext::stack(app_name).with_backend(backend),
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
			SlotTarget::new()
		],
	)
}

/// `<NamedBucket label="my-bucket"/>` — an [`S3BucketBlock`] with an explicit label,
/// non-versioned.
#[template]
pub fn NamedBucket(#[prop(into)] label: String) -> impl Bundle {
	S3BucketBlock::new(label).with_deploy_versioned(false)
}

/// `<SiteSync app_name="lambda"/>` — publish `examples/bsx_site` to the stack's
/// site bucket. The markup form of `sync_site(stack)`.
#[template]
pub fn SiteSync(#[prop(into)] app_name: String) -> impl Bundle {
	infra_ext::sync_site(&infra_ext::stack(app_name))
}

/// `<AssetsBucket/>` — the public-read, non-versioned assets bucket (`./assets`).
#[template]
pub fn AssetsBucket() -> impl Bundle {
	S3BucketBlock::new("assets")
		.with_deploy_versioned(false)
		.with_public_read(true)
}

/// `<AnalyticsTable/>` — the DynamoDB table backing the analytics store's remote
/// mode (`<app>--<stage>--analytics`, keyed by the event `id`). The deployed
/// binary reaches it via `BEET_ANALYTICS_TABLE` (set by [`FargateBeetSiteBlock`]),
/// so the created name and the runtime name agree. Resolves its [`Stack`] by
/// ancestry.
#[template]
pub fn AnalyticsTable() -> impl Bundle { DynamoTableBlock::new("analytics") }

/// `<DirSync app_name=".." bucket="site" dir="site"/>` — sync a local dir to a named
/// bucket of the stack. Generalizes [`SiteSync`] (which hardcodes `examples/bsx_site`
/// -> site bucket) to any (dir, bucket-label) pair.
#[template]
pub fn DirSync(
	#[prop(into)] app_name: String,
	#[prop(into)] bucket: String,
	#[prop(into)] dir: String,
) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	(
		S3FsStore::new(
			FsStore::new(WsPathBuf::new(dir)),
			S3BucketBlock::new(bucket)
				.with_deploy_versioned(false)
				.store(&stack),
		),
		SyncS3BucketAction,
	)
}

/// `<LambdaSiteBlock app_name="lambda" features="lambda,aws_sdk"/>` — the lambda
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

/// `<LambdaWatch app_name="lambda" timeout="30s"/>` — tail the deployed
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

/// `<LightsailSiteBlock app_name="lightsail" features="aws_sdk"/>` — the
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

/// `<LightsailWatch app_name="lightsail" timeout="30s"/>` — tail the deployed
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

/// `<FargateSiteBlock app_name="fargate"/>` — the fargate deploy block wired to
/// serve the site from the stack's bucket (`remote_env`). Named to avoid the
/// [`FargateBlock`] it builds.
#[template]
pub fn FargateSiteBlock(#[prop(into)] app_name: String) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	FargateBlock::default().with_env_vars(infra_ext::remote_env(
		infra_ext::site_bucket_name(&stack),
	))
}

/// `<FargateSshBlock app_name="ssh-site"/>` — a [`FargateBlock`] with ssh enabled and
/// the remote-site env wired from the stack's bucket. Named to avoid the
/// [`FargateBlock`] it builds.
#[template]
pub fn FargateSshBlock(#[prop(into)] app_name: String) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	FargateBlock::default().with_allow_ssh(true).with_env_vars(
		infra_ext::remote_env(infra_ext::site_bucket_name(&stack)),
	)
}

/// `<FargateWatch app_name="fargate" timeout="300s"/>` — tail the deployed
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

/// `<FargateBeetSiteBlock app_name="beet-site"/>` — the beet website's Fargate block:
/// ssh + STAGE-AWARE Cloudflare DNS + ACM, autoscaled 1..5 at 1 vCPU / 2 GB, runtime
/// env wired so the container pulls site + assets from S3 and presents one stable ssh
/// fingerprint. Bucket names derived from the stack (declared once). Markup form of
/// deploy_beet_site's `block`.
///
/// Every http hostname is PROXIED and edge-cached: Cloudflare's edge caches per
/// the origin's `CacheHeaders` and the zone rules `<CloudflareZoneSetup/>`
/// publishes. ssh lives on the DNS-only `app` hostname (`ssh app.beet.org` /
/// `ssh app.dev.beet.org`). TLS stays terminated at the origin's ACM cert,
/// which the edge verifies (the zone runs Full strict).
///
/// SAFETY / stage-aware DNS (a deliberate change from the original, which always
/// published all three hostnames): `dev` publishes ONLY `dev.beet.org`; `prod`
/// publishes the production apex `beet.org` + `www.beet.org`. This is REQUIRED so a
/// `dev` deploy never touches production apex DNS, and it makes `--stage` meaningful.
#[template]
pub fn FargateBeetSiteBlock(#[prop(into)] app_name: String) -> impl Bundle {
	let stack = infra_ext::stack(&app_name);
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();
	let site_bucket = S3BucketBlock::new("site")
		.with_deploy_versioned(false)
		.store(&stack)
		.bucket_name()
		.to_string();
	let assets_bucket = S3BucketBlock::new("assets")
		.with_deploy_versioned(false)
		.store(&stack)
		.bucket_name()
		.to_string();
	// the analytics DynamoDB table name, the same value `<AnalyticsTable/>` creates.
	let analytics_table = DynamoTableBlock::new("analytics").table_name(&stack);
	let block = FargateBlock::default()
		.with_allow_ssh(true)
		.with_max_count(5)
		.with_cpu(1024)
		.with_memory(2048)
		.with_static_env("BEET_SERVICE_ACCESS", "remote")
		.with_static_env("BEET_SITE_BUCKET", site_bucket)
		.with_static_env("BEET_ASSETS_BUCKET", assets_bucket)
		.with_static_env("BEET_ANALYTICS_TABLE", analytics_table)
		.with_static_env("BEET_SSH_HOST_KEY", ssh_host_key);
	// prod claims the apex + www (proxied, edge-cached) plus the DNS-only `app`
	// hostname carrying ssh + future live apps; other stages get their
	// subdomain (proxied) + `app.dev` (DNS-only ssh).
	if stack.is_production() {
		block
			.with_dns(
				DnsProvider::cloudflare("beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(
				DnsProvider::cloudflare("www.beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(DnsProvider::cloudflare("app.beet.org", zone_id.clone()))
	} else {
		block
			.with_dns(
				DnsProvider::cloudflare("dev.beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(DnsProvider::cloudflare(
				"app.dev.beet.org",
				zone_id.clone(),
			))
	}
}

/// `<BeetSiteDeployHost>` — the [`Stack`]-bearing parent for the beet-site deploy
/// routes, mounted inside the root dev host so the routes resolve a [`Stack`] by
/// ancestry WITHOUT a second `CliServer`/`Router` (the root already provides those).
/// Carries the standard IaC verb routes (validate/plan/apply/show/list/destroy/...)
/// so `just beet validate`/`destroy` operate on the beet site, and a slot the
/// declared `<Route>` deploy/sync/watch children land in.
#[template]
pub fn BeetSiteDeployHost() -> impl Bundle {
	(infra_ext::stack("beet-site"), children![
		Validate,
		Plan,
		Apply,
		Show,
		List,
		Destroy,
		Rollback,
		Rollforward,
		SlotTarget::new(),
	])
}
