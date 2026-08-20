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
	infra_ext::beet_cargo_build(features).into_build_artifact()
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

/// `<SiteSync app_name="lambda"/>` — publish `examples/bsx_site` to the stack's
/// app bucket. The markup form of `sync_site(stack)`.
#[template]
pub fn SiteSync(#[prop(into)] app_name: String) -> impl Bundle {
	infra_ext::sync_site(&infra_ext::stack(app_name))
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
				infra_ext::app_bucket_name(&stack),
			)?)
			.into_lambda_build_artifact()?,
	)
		.xok()
}

/// `<LambdaWatch timeout="30s"/>` — tail the deployed lambda's logs. The log
/// group composes from the ancestor [`Stack`] when the tail runs, so nothing
/// here restates the app identity.
#[template]
pub fn LambdaWatch(timeout: Option<Duration>) -> impl Bundle {
	infra_ext::watch(
		WatchTarget::Lambda(LambdaBlock::default().label().clone()),
		timeout,
	)
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
			infra_ext::app_bucket_name(&stack),
		)?),
		infra_ext::beet_cargo_build(features).into_build_artifact(),
	)
		.xok()
}

/// `<LightsailWatch timeout="30s"/>` — tail the deployed instance's logs, the
/// same group its cloud-init agent forwards to. Resolves the [`Stack`] by
/// ancestry when the tail runs.
#[template]
pub fn LightsailWatch(timeout: Option<Duration>) -> impl Bundle {
	infra_ext::watch(
		WatchTarget::Lightsail(LightsailBlock::default().label().clone()),
		timeout,
	)
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
			infra_ext::app_bucket_name(&stack),
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

/// `<FargateWatch timeout="300s"/>` — tail the deployed service's logs.
/// Resolves the [`Stack`] by ancestry when the tail runs.
#[template]
pub fn FargateWatch(timeout: Option<Duration>) -> impl Bundle {
	infra_ext::watch(WatchTarget::Fargate, timeout)
}

/// `<LightsailBeetSiteBlock features="aws_sdk,ssh,geoip"/>` —
/// the beet website's Lightsail block plus its build artifact, on one entity
/// (paired by `TofuApplyAction`, see [`LambdaSiteBlock`]): one `small_3_0` box
/// (2 GB, known monthly price, no NLB) serving http behind Caddy and the beet
/// ssh TUI on port 22, with STAGE-AWARE Cloudflare DNS at the static IP.
/// Runtime env wired so the binary reads its one app bucket (site and assets
/// alike) and presents one stable ssh fingerprint. Bucket names derived from
/// the stack (declared once).
///
/// Every http hostname is PROXIED and edge-cached (an `A` record at the static
/// IP): Cloudflare's edge caches per the origin's `CacheHeaders` and the zone
/// rules `<CloudflareZoneSetup/>` publishes. ssh lives on the DNS-only `app`
/// hostname (`ssh app.beet.org` / `ssh app.dev.beet.org`), since Cloudflare
/// does not proxy raw TCP. Caddy terminates TLS at the origin with a Let's
/// Encrypt cert covering every hostname, which the edge verifies (the zone
/// runs Full strict). The box's own sshd moves to port 2222, reachable with
/// the stack's key pair.
///
/// SAFETY / stage-aware DNS: `dev` publishes ONLY `dev.beet.org`; `prod`
/// publishes the production apex `beet.org` + `www.beet.org`. This is REQUIRED so a
/// `dev` deploy never touches production apex DNS, and it makes `--stage` meaningful.
#[template(system)]
pub fn LightsailBeetSiteBlock(
	#[prop(into)] features: String,
	/// The boot route the unit dispatches: the site entry is a `CliServer`
	/// dispatcher, so the deployed process names which of its routes IS the site.
	#[prop(default = String::from("serve"))]
	exec_route: String,
	scopes: ResourceScopeQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	// the app identity comes from the app's own `<PackageConfig/>`, resolved
	// through the one composition both the deploy and the runtime read.
	let scope = scopes.get(entity)?;
	let is_production =
		scopes.stack(entity).is_some_and(|stack| stack.is_production());
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();
	let app_bucket = scope.resource_name("app");
	let block = LightsailBlock::default()
		.with_bundle_id("small_3_0")
		.with_allow_ssh(true)
		.with_exec_route(exec_route)
		// one declaration for both channels: the block splits boot selection (the
		// entry store, the transports) onto the unit's `ExecStart` and the service
		// config the runtime reads onto its `Environment=` lines. The analytics
		// table is NOT here: the site declares it, and the deployed process
		// resolves the same declaration.
		.with_bootstrap(BootstrapConfig {
			store: Some(StoreUri::parse(&format!("s3://{app_bucket}"))?),
			server: Some(ServerFilter::new("http,ssh")),
			service_access: ServiceAccess::Remote,
			..default()
		})
		// private key material: its own channel, so no renderer can ever put it
		// on an argv line.
		.with_secret_env("BEET_SSH_HOST_KEY", ssh_host_key);
	// prod claims the apex + www (proxied, edge-cached) plus the DNS-only `app`
	// hostname carrying ssh + future live apps; other stages get their
	// subdomain (proxied) + `app.dev` (DNS-only ssh).
	let block = if is_production {
		block
			.with_dns(
				DnsProvider::cloudflare("beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(
				DnsProvider::cloudflare("www.beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(DnsProvider::cloudflare("app.beet.org", zone_id))
	} else {
		block
			.with_dns(
				DnsProvider::cloudflare("dev.beet.org", zone_id.clone())
					.with_proxied(true),
			)
			.with_dns(DnsProvider::cloudflare("app.dev.beet.org", zone_id))
	};
	(
		block,
		infra_ext::beet_cargo_build(features).into_build_artifact(),
	)
		.xok()
}

/// `<DeployHost>` — the [`Stack`]-bearing parent for an app's deploy routes,
/// mounted inside a dispatch router so the routes resolve a [`Stack`] by
/// ancestry WITHOUT a second `CliServer`/`Router` (the entry already provides
/// those). Carries the standard IaC verb routes
/// (validate/plan/apply/show/list/destroy/...) and a slot the declared `<Route>`
/// deploy/sync/watch children land in.
///
/// The app identity comes from the application's own `<PackageConfig
/// app_name=".."/>`, the ONE place it lives, so a deploy can never name a
/// different app than the process it deploys. `app_name` overrides it for a
/// multi-app entry.
///
/// `stage` overrides the stack's stage (which otherwise flows from `--stage`):
/// `<Route path="shared"><DeployHost stage="shared">..` hosts the shared-stage
/// resources (the assets bucket) with the same verbs under the `shared/` route
/// prefix, so provisioning them is its own step (`beet shared apply`), separate
/// from any stage deploy.
#[template(system)]
pub fn DeployHost(
	#[prop(default)] stage: Option<String>,
	#[prop(default)] app_name: Option<String>,
	package: Option<Res<PackageConfig>>,
) -> Result<impl Bundle> {
	let app_name = match app_name {
		Some(app_name) => SmolStr::from(app_name),
		None => ResourceScope::from_package(package.as_deref())?
			.app_name()
			.clone(),
	};
	let stack = match stage {
		Some(stage) => infra_ext::stack(app_name).with_stage(stage),
		None => infra_ext::stack(app_name),
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
		.xok()
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

	/// One declaration, two meanings. The site declares its analytics table
	/// once, at router level; the DEPLOY provisions that table into the app
	/// stack's tofu config, and the RUNTIME attaches a store for the same
	/// resolved name off the same entity. The invariant is that the two strings
	/// agree, for any stage.
	///
	/// REGRESSION: the two sides used to derive the name independently, and
	/// `site/main.bsx` declared no app name, so the runtime fell back to the
	/// kebab-cased title and wrote to `beet--<stage>--analytics`, a table the
	/// deploy never creates. Every event on the live site failed with a DynamoDB
	/// `ResourceNotFoundException` while the site served perfectly and the
	/// summary reported `0 events` on a green deploy.
	#[beet_core::test]
	fn one_declaration_names_the_table_for_both_sides() {
		let mut world = test_world();
		world.insert_resource(PackageConfig {
			app_name: Some("beet-site".into()),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		// the declaration sits OUTSIDE the deploy host, since its reason to
		// exist is the runtime meaning; the stage host adopts it while the
		// shared host (a stage override) never does.
		spawn_markup(
			&mut world,
			router,
			r#"<Fragment>
				<DynamoTableBlock label="analytics"/>
				<DeployHost/>
				<Route path="shared"><DeployHost stage="shared"/></Route>
			</Fragment>"#,
		);
		let scope = ResourceScope::new("beet-site", "dev");
		let expected = scope.resource_name("analytics");

		/// The tofu json a host stack builds.
		fn config_json(world: &mut World, host: Entity) -> String {
			world
				.with_state::<StackQuery, _>(|stacks| {
					stacks.build_config(host).map(|(_, config)| config)
				})
				.unwrap()
				.to_json()
				.to_string()
		}
		let hosts = world
			.query::<(Entity, &Stack)>()
			.iter(&world)
			.map(|(entity, stack)| (entity, stack.stage().clone()))
			.collect::<Vec<_>>();
		for (host, stage) in hosts {
			let json = config_json(&mut world, host);
			// the deploy side: the table lands in the app stack's config only
			match stage.as_str() {
				"shared" => json.as_str().xnot().xpect_contains("analytics"),
				_ => json.as_str().xpect_contains(&expected),
			};
		}

		// the runtime side: the same composition off the declaration entity
		world
			.query::<&DynamoTableBlock>()
			.single(&world)
			.unwrap()
			.table_name(&scope)
			.xpect_eq(expected);
	}

	/// The `shared`-stage host, the shape the site entry declares: its verb
	/// routes nest under the `shared/` prefix, the assets bucket resolves the
	/// shared stack by ancestry (`beet-site--shared--assets`), and the app name
	/// comes from the app's own `<PackageConfig/>` rather than a prop.
	#[beet_core::test]
	fn shared_host_prefixes_verbs_and_names_bucket() {
		let mut world = test_world();
		world.insert_resource(PackageConfig {
			app_name: Some("beet-site".into()),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Route path="shared">
				<DeployHost stage="shared">
					<S3BucketBlock label="assets" deploy_versioned=false public_read=true force_destroy=false/>
					<Route path="push" {ExchangeSequence}>
						<DirSync bucket="assets" local_dir="site/assets"/>
					</Route>
					<Route path="pull" {ExchangeSequence}>
						<DirSync bucket="assets" local_dir="site/assets" {SyncS3Bucket{direction:Pull, no_sign_request:true}}/>
					</Route>
				</DeployHost>
			</Route>"#,
		);
		let tree = world.entity(router).get::<RouteTree>().unwrap();
		tree.find(&["shared", "validate"]).xpect_some();
		tree.find(&["shared", "apply"]).xpect_some();
		tree.find(&["shared", "push"]).xpect_some();
		tree.find(&["shared", "pull"]).xpect_some();
		// the bucket block resolved the shared-stage stack by ancestry
		world
			.query::<&S3Store>()
			.single(&world)
			.unwrap()
			.bucket_name()
			.xpect_eq("beet-site--shared--assets");
		// ..and so did the syncs, which name the bucket by label alone
		world
			.query::<&S3FsStore>()
			.iter(&world)
			.map(|store| store.s3_store().bucket_name().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"beet-site--shared--assets".to_string(),
				"beet-site--shared--assets".to_string(),
			]);
	}

	/// The apply layer coerces from markup, so a deploy route can author its
	/// layered applies (`layer="storage"` before the content sync, a bare full
	/// apply after it) as the same tag.
	#[beet_core::test]
	fn tofu_apply_layer_prop() {
		let mut world = test_world();
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Fragment><TofuApply layer="storage"/><TofuApply/></Fragment>"#,
		);
		world
			.query::<&TofuApply>()
			.iter(&world)
			.map(|apply| apply.layer().clone())
			.collect::<Vec<_>>()
			.xpect_eq(vec![Some(SmolStr::new("storage")), None]);
	}

	/// The sync verbs coerce from markup: an enum by variant name, the flags by
	/// bool, and the defaults are the conservative push/additive pair.
	#[beet_core::test]
	fn dir_sync_verb_props() {
		/// The single [`SyncS3Bucket`] a `<DirSync>` markup fragment builds.
		fn sync(markup: &str) -> SyncS3Bucket {
			let mut world = test_world();
			world.insert_resource(PackageConfig {
				app_name: Some("beet-site".into()),
				..default()
			});
			let router = world.spawn(Router::with_defaults()).id();
			spawn_markup(&mut world, router, markup);
			world.query::<&SyncS3Bucket>().single(&world).unwrap().clone()
		}
		let pull = sync(
			r#"<DirSync bucket="assets" local_dir="assets" {SyncS3Bucket{direction:Pull, no_sign_request:true}}/>"#,
		);
		pull.direction().xpect_eq(SyncDirection::Pull);
		pull.no_sign_request().xpect_true();
		pull.delete().xpect_false();
		let push = sync(
			r#"<DirSync bucket="app" local_dir="site" {SyncS3Bucket{delete:true}}/>"#,
		);
		push.direction().xpect_eq(SyncDirection::Push);
		push.delete().xpect_true();
	}
}
