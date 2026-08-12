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

/// `<StackHost app_name="lambda">` — the IaC deployer host: a one-shot
/// [`CliServer`] owning the boot, with the default router as its dispatch child
/// carrying the [`Stack`] (so the blocks + verbs resolve it by ancestry), the
/// standard IaC verb routes (validate/plan/apply/...), and a slot for the
/// example's own deploy/sync/watch routes. The markup form of
/// `(stack(name), Stack::cli())`.
#[template]
pub fn StackHost(#[prop(into)] app_name: String) -> impl Bundle {
	(CliServer::default(), children![(
		infra_ext::stack(app_name),
		Router::with_defaults(),
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
	)])
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
		Router::with_defaults(),
		children![
			CliServer::default(),
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

/// `<AssetsBucket/>` — the assets bucket: the source of record for files too
/// large for git, public-read so a fresh checkout can `pull-assets` without
/// credentials. Declared under the `shared`-stage host (`app--shared--assets`):
/// a source is shared by developers rather than owned by any deploy stage, so it
/// is provisioned by the shared stack's own verbs (`beet shared apply`) and no
/// stage deploy or destroy touches it. As a source of record it refuses a
/// non-empty delete (`force_destroy=false`).
#[template]
pub fn AssetsBucket() -> impl Bundle {
	S3BucketBlock::new("assets")
		.with_deploy_versioned(false)
		.with_public_read(true)
		.xmap(|mut bucket| {
			bucket.force_destroy = Some(false);
			bucket
		})
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
/// -> site bucket) to any (dir, bucket-label) pair. `stage` overrides the stack
/// stage (which otherwise flows from `--stage`), eg `stage="shared"` to push the
/// shared assets bucket.
#[template]
pub fn DirSync(
	#[prop(into)] app_name: String,
	#[prop(into)] bucket: String,
	#[prop(into)] dir: String,
	stage: Option<String>,
) -> impl Bundle {
	let stack = match stage {
		Some(stage) => infra_ext::stack(&app_name).with_stage(stage),
		None => infra_ext::stack(&app_name),
	};
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
/// deploy block plus its build artifact, on one entity. They share an entity
/// because `TofuApplyAction` pairs the `BuildArtifact` with the block on the same
/// entity to upload it under the block's label, the S3 key the lambda reads its
/// code from. The lambda runtime offers no argv, so the site-store args
/// (`remote_bootstrap`) bake into the zip's `bootstrap` script (the env-to-args
/// boundary). The markup form of the rust example's
/// `(block, build_beet_lambda_binary(features))` deploy child.
#[template]
pub fn LambdaSiteBlock(
	#[prop(into)] app_name: String,
	#[prop(into)] features: String,
) -> Result<impl Bundle> {
	let stack = infra_ext::stack(&app_name);
	(
		LambdaBlock::default(),
		infra_ext::beet_cargo_build(features)
			.with_bootstrap(infra_ext::remote_bootstrap(
				infra_ext::site_bucket_name(&stack),
			)?)
			.into_lambda_build_artifact()?,
	)
		.xok()
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
/// lightsail deploy block (its systemd `ExecStart` launches the binary with the
/// site-store config, `remote_bootstrap`) plus its build artifact, on one entity
/// (paired by `TofuApplyAction`, see [`LambdaSiteBlock`]). The markup form of
/// `(block, build_beet_binary(features))`.
#[template]
pub fn LightsailSiteBlock(
	#[prop(into)] app_name: String,
	#[prop(into)] features: String,
) -> Result<impl Bundle> {
	let stack = infra_ext::stack(&app_name);
	(
		LightsailBlock::default().with_bootstrap(infra_ext::remote_bootstrap(
			infra_ext::site_bucket_name(&stack),
		)?),
		infra_ext::beet_cargo_build(features).into_build_artifact(),
	)
		.xok()
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
/// serve the site from the stack's bucket: the site-store config
/// (`remote_bootstrap`) lands in the container `CMD` via the sibling
/// `<BuildDockerImage/>`. Named to avoid the [`FargateBlock`] it builds.
#[template]
pub fn FargateSiteBlock(
	#[prop(into)] app_name: String,
) -> Result<impl Bundle> {
	let stack = infra_ext::stack(&app_name);
	FargateBlock::default()
		.with_bootstrap(infra_ext::remote_bootstrap(
			infra_ext::site_bucket_name(&stack),
		)?)
		.xok()
}

/// `<FargateSshBlock/>` — a [`FargateBlock`] with ssh enabled. No site-store
/// wiring: its deploy bakes a specific example binary into the image rather than
/// serving a synced bucket. Named to avoid the [`FargateBlock`] it builds.
#[template]
pub fn FargateSshBlock() -> impl Bundle {
	FargateBlock::default().with_allow_ssh(true)
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
pub fn FargateBeetSiteBlock(
	#[prop(into)] app_name: String,
) -> Result<impl Bundle> {
	let stack = infra_ext::stack(&app_name);
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();
	let site_bucket = S3BucketBlock::new("site")
		.with_deploy_versioned(false)
		.store(&stack)
		.bucket_name()
		.to_string();
	// the assets bucket lives on the `shared` stage: a source of record shared
	// by developers, not owned by any deploy stage (see `AssetsBucket`).
	let assets_bucket = S3BucketBlock::new("assets")
		.with_deploy_versioned(false)
		.store(&infra_ext::stack(&app_name).with_stage("shared"))
		.bucket_name()
		.to_string();
	// the analytics DynamoDB table name, the same value `<AnalyticsTable/>` creates.
	let analytics_table = DynamoTableBlock::new("analytics").table_name(&stack);
	let block = FargateBlock::default()
		.with_allow_ssh(true)
		.with_max_count(5)
		.with_cpu(1024)
		.with_memory(2048)
		// one declaration for both channels: the block splits boot selection (the
		// entry store, the transports) onto the container `CMD` and the service
		// config the runtime `PackageConfig`/analytics store read onto task env.
		.with_bootstrap(BootstrapConfig {
			store: Some(StoreUri::parse(&format!("s3://{site_bucket}"))?),
			server: Some(ServerFilter::new("http,ssh")),
			service_access: ServiceAccess::Remote,
			analytics_table: Some(analytics_table.into()),
			..default()
		})
		// interim, dies with the store-topology phase (`AssetsStore` is deleted
		// there, and the app bucket physically contains `assets/`).
		.with_assets_bucket(assets_bucket)
		// private key material: its own channel, so no renderer can ever put it
		// on an argv line or in a `CMD` array.
		.with_secret_env("BEET_SSH_HOST_KEY", ssh_host_key);
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
	.xok()
}

/// `<BeetSiteDeployHost>` — the [`Stack`]-bearing parent for the beet-site deploy
/// routes, mounted inside the root dev host so the routes resolve a [`Stack`] by
/// ancestry WITHOUT a second `CliServer`/`Router` (the root already provides those).
/// Carries the standard IaC verb routes (validate/plan/apply/show/list/destroy/...)
/// so `just beet validate`/`destroy` operate on the beet site, and a slot the
/// declared `<Route>` deploy/sync/watch children land in.
///
/// `stage` overrides the stack's stage (which otherwise flows from `--stage`):
/// `<Route path="shared"><BeetSiteDeployHost stage="shared">..` hosts the
/// shared-stage resources (the assets bucket) with the same verbs under the
/// `shared/` route prefix, so provisioning them is its own step
/// (`beet shared apply`), separate from any stage deploy.
#[template]
pub fn BeetSiteDeployHost(stage: Option<String>) -> impl Bundle {
	let stack = match stage {
		Some(stage) => infra_ext::stack("beet-site").with_stage(stage),
		None => infra_ext::stack("beet-site"),
	};
	(stack, children![
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::infra::InfraExamplesPlugin;

	fn test_world() -> World {
		(
			AsyncPlugin,
			TemplatePlugin,
			DocumentPlugin,
			RouterPlugin,
			InfraExamplesPlugin,
		)
			.into_world()
	}

	fn spawn_markup(world: &mut World, router: Entity, markup: &str) {
		let nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		world
			.spawn(ChildOf(router))
			.insert_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap();
		world.flush();
	}

	/// The `shared`-stage host, the shape `main.bsx` declares: its verb routes
	/// nest under the `shared/` prefix, and the assets bucket resolves the
	/// shared stack by ancestry (`beet-site--shared--assets`).
	#[beet_core::test]
	fn shared_host_prefixes_verbs_and_names_bucket() {
		let mut world = test_world();
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Route path="shared">
				<BeetSiteDeployHost stage="shared">
					<AssetsBucket/>
					<Route path="push" {ExchangeSequence}>
						<DirSync app_name="beet-site" bucket="assets" dir="assets" stage="shared"/>
					</Route>
				</BeetSiteDeployHost>
			</Route>"#,
		);
		let tree = world.entity(router).get::<RouteTree>().unwrap();
		tree.find(&["shared", "validate"]).xpect_some();
		tree.find(&["shared", "apply"]).xpect_some();
		tree.find(&["shared", "push"]).xpect_some();
		// the bucket block resolved the shared-stage stack by ancestry
		world
			.query::<&S3Store>()
			.single(&world)
			.unwrap()
			.bucket_name()
			.xpect_eq("beet-site--shared--assets");
	}

	/// `<DirSync stage="shared"/>` overrides the argv stage; the default stays
	/// stage-scoped.
	#[beet_core::test]
	fn dir_sync_stage_prop() {
		let mut world = test_world();
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<DirSync app_name="beet-site" bucket="assets" dir="assets" stage="shared"/>"#,
		);
		world
			.query::<&S3FsStore>()
			.single(&world)
			.unwrap()
			.s3_store()
			.bucket_name()
			.xpect_eq("beet-site--shared--assets");
	}
}
