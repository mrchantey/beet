//! Markup wrappers for the infra example types that are not directly spawnable.
//!
//! Most deploy types are reflect components spawned directly by tag (eg
//! `<Stack/>`, `<S3BucketBlock/>`, `<CloudflareWorkerBlock/>`). The wrappers here
//! cover the rest: types that build a non-`Reflect` value (a [`BuildArtifact`]'s
//! `ChildProcess`, an action route bundle) or compute stack-derived config. A thin
//! `#[template]` wraps each: its props struct is reflect-registered, its body
//! builds the bundle.
//!
//! None of them carries resource identity. That belongs to `<Stack>`, a component
//! registered in every native build: a stage or app name on a template prop is
//! absent exactly in the binary that did not link the template.
use crate::infra::infra_ext;
use beet_core::prelude::*;
use beet_infra::prelude::*;

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

/// `<DeployRoutes/>` — the standard IaC verb routes (validate / plan / apply /
/// show / list / destroy / rollback / rollforward), the markup form of
/// [`Stack::verbs`].
///
/// It carries no identity of its own: each verb resolves its [`Stack`] by
/// ancestry, so authoring it under a `<Stack>` is the whole declaration. That
/// separation is the point — a stage or app name riding a TEMPLATE prop is
/// absent from any binary that did not link the template, which is exactly how a
/// `shared` scope can go missing in a lean build.
#[template]
pub fn DeployRoutes() -> impl Bundle { Stack::verbs() }

/// `<StateBackendToggle/>` — select this launch's tofu state backend from argv:
/// S3 when `--s3-backend` is passed, local otherwise. The markup form of
/// lifecycle.rs's backend toggle.
///
/// The backend is a property of the LAUNCH, not of a stack's identity, so the
/// toggle lands on the process [`Deployment`] and nothing about it is authored
/// per stack.
#[template(system)]
pub fn StateBackendToggle(mut deployment: ResMut<Deployment>) {
	let backend: StackBackend =
		if CliArgs::parse_env().params.contains_key("s3-backend") {
			S3Backend::default().into()
		} else {
			LocalBackend::default().into()
		};
	deployment.set_backend(backend);
}

/// `<SiteSync/>` — publish `examples/bsx_site` to the stack's app bucket. The
/// markup form of `sync_site(stack, deployment)`: the bucket is deploy-versioned,
/// so the sync also needs this launch's id.
///
/// The bucket name composes from the ancestor `<Stack>`, so nothing here
/// restates the app identity the stack already answers.
#[template(system)]
pub fn SiteSync(
	stacks: StackQuery,
	deployment: Res<Deployment>,
	entity: Entity,
) -> impl Bundle {
	infra_ext::sync_site(&stacks.resolve(entity), &deployment)
}

/// An **opinionated** block for websites built with lambda.
/// `<LambdaSiteBlock features="lambda,aws_sdk"/>` — the lambda deploy block plus
/// its build artifact, on one entity. They share an entity because
/// `TofuApplyAction` pairs the `BuildArtifact` with the block on the same entity
/// to upload it under the block's label, the S3 key the lambda reads its code
/// from. The lambda runtime offers no argv, so the site-store args
/// (`remote_bootstrap`) bake into the zip's `bootstrap` script (the env-to-args
/// boundary). The markup form of the rust example's
/// `(block, build_beet_lambda_binary(features))` deploy child.
///
/// `authorities` publishes the site's public hostnames as api gateway custom
/// domains behind Cloudflare's edge (proxied, so they are cached and the origin
/// is never addressed directly). Without them the function answers only on the
/// gateway's default `execute-api` endpoint.
///
/// SAFETY / stage-aware DNS: those are PRODUCTION names, so only a production
/// stage publishes them; any other stage deploys the same function reachable
/// only at its own gateway endpoint. This is REQUIRED so a `dev` deploy never
/// takes the production apex, and it makes `--stage` meaningful.
#[template(system)]
pub fn LambdaSiteBlock(
	#[prop(into)] features: String,
	/// Comma-separated public hostnames, the first being the certificate's
	/// primary domain, eg `beetmash.com,www.beetmash.com`. Reads the zone from
	/// `CLOUDFLARE_ZONE_ID`.
	#[prop(into, default)]
	authorities: String,
	/// The route the deployed entry dispatches, for an entry that is its own CLI
	/// and so names which of its verbs IS the site.
	#[prop(into)]
	exec_route: Option<String>,
	/// The beet checkout to build the binary out of, workspace-relative. A site
	/// repo carrying no beet crates of its own names one; a site inside the beet
	/// workspace leaves it unset.
	#[prop(into)]
	workspace_dir: Option<String>,
	stacks: StackQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	// the ancestor `<Stack>`'s identity: one composition behind both the code
	// bucket and the stage-aware DNS.
	let stack = stacks.resolve(entity);
	let is_production = stack.is_production();
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
	let block = authorities
		.split(',')
		.map(str::trim)
		.filter(|authority| is_production && !authority.is_empty())
		.fold(LambdaBlock::default(), |block, authority| {
			block.with_dns(
				DnsProvider::cloudflare(authority, zone_id.clone())
					.with_proxied(true),
			)
		});
	let mut build = infra_ext::beet_cargo_build(features).with_bootstrap(
		infra_ext::remote_bootstrap(infra_ext::app_bucket_name(&stack))?,
	);
	if let Some(exec_route) = exec_route {
		build = build.with_exec_route(exec_route);
	}
	if let Some(workspace_dir) = workspace_dir {
		build =
			build.with_workspace_dir(WsPathBuf::new(workspace_dir).into_abs());
	}
	(block, build.into_lambda_build_artifact()?).xok()
}

/// `<LambdaJobBlock label="rollup" features="aws_sdk,lambda" exec_route="jobs"/>`
/// — an INVOKE-ONLY lambda plus its build artifact, on one entity (paired by
/// `TofuApplyAction`, see [`LambdaSiteBlock`]): the target a
/// `<ScheduledJobBlock/>` drives.
///
/// The counterpart of [`LambdaSiteBlock`] for work rather than serving, and the
/// difference is the whole point: it publishes no function url, no api gateway
/// and no hostname, so nothing but its declared invoker can reach it. A batch
/// job that scans a table and rewrites every row in it has no business behind an
/// endpoint whose authorization is `NONE`.
///
/// Otherwise it boots exactly as a served lambda does: the runtime offers no
/// argv, so the entry-store config (`remote_bootstrap`) bakes into the zip's
/// `bootstrap` script and `exec_route` names the verb it launches. That verb
/// hosts the router the schedule's invoke is dispatched into, so what runs is a
/// route of the same entry document the site serves from.
#[template(system)]
pub fn LambdaJobBlock(
	/// The function's label, which the schedule's `target` names.
	#[prop(into)]
	label: String,
	#[prop(into)] features: String,
	/// The entry verb the function boots, whose router hosts the job routes an
	/// invoke dispatches.
	#[prop(into)]
	exec_route: String,
	/// Seconds one run may take, the service maximum by default: a job sweeps a
	/// store rather than answering a request, and the first run over a history
	/// that predates it is the longest one it will ever do.
	#[prop(default = 900)]
	timeout_secs: i64,
	stacks: StackQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	let stack = stacks.resolve(entity);
	(
		LambdaBlock::default()
			.with_label(label)
			.with_http(false)
			.with_timeout_secs(timeout_secs),
		infra_ext::beet_cargo_build(features)
			.with_bootstrap(infra_ext::remote_bootstrap(
				infra_ext::app_bucket_name(&stack),
			)?)
			.with_exec_route(exec_route)
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

/// `<LightsailSiteBlock features="aws_sdk"/>` — the lightsail deploy block (its
/// systemd `ExecStart` launches the binary with the site-store config,
/// `remote_bootstrap`) plus its build artifact, on one entity (paired by
/// `TofuApplyAction`, see [`LambdaSiteBlock`]). The markup form of
/// `(block, build_beet_binary(features))`. The bucket it serves from composes
/// from the ancestor `<Stack>`.
#[template(system)]
pub fn LightsailSiteBlock(
	#[prop(into)] features: String,
	stacks: StackQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	let stack = stacks.resolve(entity);
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
		WatchTarget::Instance(LightsailBlock::default().label().clone()),
		timeout,
	)
}

/// `<FargateSiteBlock/>` — the fargate deploy block wired to serve the site from
/// the stack's bucket: the site-store config (`remote_bootstrap`) lands in the
/// container `CMD` via the sibling `<BuildDockerImage/>`. Named to avoid the
/// [`FargateBlock`] it builds. The bucket composes from the ancestor `<Stack>`.
#[template(system)]
pub fn FargateSiteBlock(
	stacks: StackQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	let stack = stacks.resolve(entity);
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
	stacks: StackQuery,
	entity: Entity,
) -> Result<impl Bundle> {
	// the app identity comes from the ancestor `<Stack>`, else the app's own
	// `<PackageConfig/>`: one composition both the deploy and the runtime read.
	let stack = stacks.resolve(entity);
	let is_production = stack.is_production();
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID").unwrap_or_default();
	let ssh_host_key = env_ext::var("BEET_SSH_HOST_KEY").unwrap_or_default();
	let app_bucket = stack.resource_name("app");
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
			server: Some(RunningSetFilter::new("http,ssh")),
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

#[cfg(test)]
mod test {
	use super::*;
	use crate::infra::InfraExamplesPlugin;
	use beet_net::prelude::*;
	use beet_router::prelude::*;

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

	/// A block declared outside every `<Stack>` resolves the process default
	/// rather than raising: it belongs to no deploy's config, but its RUNTIME
	/// meaning still names the resource this process would.
	///
	/// REGRESSION: `examples/infra/bucket.bsx` and `lambda.bsx` both HUNG. Their
	/// `<S3BucketBlock/>` sat in a host template's slot, so at attach time it was
	/// still parented to the unresolved element and had no ancestor `<Stack>`;
	/// resolution used to `bevybail!` there, and a raise in a queued command left
	/// the process with nothing to run and no output.
	#[beet_core::test]
	fn a_stackless_block_resolves_the_default_stack() {
		let mut world = test_world();
		world.insert_resource(PackageConfig {
			app_name: "bucket-example".into(),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Route path="run" {ExchangeSequence}>
				<S3BucketBlock label="my-bucket" deploy_versioned=false/>
			</Route>"#,
		);
		world
			.query::<&S3Store>()
			.single(&world)
			.unwrap()
			.bucket_name()
			.xpect_eq("bucket-example--dev--my-bucket");
	}

	/// A block declared under an explicit `<Stack>` resolves THAT stack, through
	/// the `<Route>` it is nested in: the shape every entry now authors, where
	/// identity is a registered component on the tree rather than a prop on a
	/// template the binary may not have linked.
	#[beet_core::test]
	fn a_stack_ancestor_names_the_block() {
		let mut world = test_world();
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Stack app_name="bucket-example" stage="staging">
				<Route path="run" {ExchangeSequence}>
					<S3BucketBlock label="my-bucket" deploy_versioned=false/>
				</Route>
			</Stack>"#,
		);
		world
			.query::<&S3Store>()
			.single(&world)
			.unwrap()
			.bucket_name()
			.xpect_eq("bucket-example--staging--my-bucket");
	}

	/// One declaration, two meanings. The site declares its analytics table
	/// once, under its stage `<Stack>`; the DEPLOY provisions that table into
	/// that stack's tofu config, and the RUNTIME attaches a store for the same
	/// resolved name off the same entity. The invariant is that the two strings
	/// agree, for any stage.
	///
	/// A resource belongs to the stack it is authored under and to no other, so
	/// the `shared` stack (whose reason to exist is resources no stage deploy
	/// owns) never provisions it. That used to be inferred: an unscoped block was
	/// ADOPTED by whichever host matched the process stage, which is one
	/// misplaced declaration away from provisioning into the wrong stack.
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
			app_name: "beet-site".into(),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Fragment>
				<Stack>
					<DynamoTableBlock label="analytics"/>
				</Stack>
				<Route path="shared"><Stack stage="shared"/></Route>
			</Fragment>"#,
		);
		let scope = Stack::new("beet-site")
			.with_stage("dev")
			.resolve(&PackageConfig::default());
		let expected = scope.resource_name("analytics");

		/// The tofu json a stack builds.
		fn config_json(world: &mut World, root: Entity) -> String {
			let (.., config) =
				RenderScope::render(world, root).unwrap().finish().unwrap();
			config.to_json_string().unwrap()
		}
		let stacks = world
			.query::<(Entity, &Stack)>()
			.iter(&world)
			.map(|(entity, stack)| {
				(
					entity,
					stack
						.resolve(&PackageConfig::default())
						.stage()
						.to_string(),
				)
			})
			.collect::<Vec<_>>();
		stacks.len().xpect_eq(2);
		for (root, stage) in stacks {
			let json = config_json(&mut world, root);
			// the deploy side: the table lands in the stage stack's config only
			match stage.as_str() as &str {
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
		// ..which locally is backed by a workspace directory rather than the
		// remote table, so one declaration runs both ways
		world.query::<&FsStore>().single(&world).xpect_ok();
	}

	/// The analytics retention stack the site entry declares, end to end: the
	/// events table that expires, the aggregate table that does not, the bucket
	/// the raws are archived into, the invoke-only function and the timer that
	/// drives it — plus the job itself, bound to all three stores by relation.
	///
	/// The deploy has to RENDER, not just spawn: an unpointed schedule and a
	/// hostname on a gateway-less function are both render-time failures, and a
	/// stack whose tags resolved to nothing deploys green with no timer in it.
	#[beet_core::test]
	fn the_rollup_declares_its_stores_its_timer_and_its_job() {
		let mut world = test_world();
		world.insert_resource(PackageConfig {
			app_name: "beet-site".into(),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Fragment>
				<Route path="jobs" {HttpServer}>
					<Router>
						<Route path="rollup" {(AnalyticsRollupJob, StoreRef($analytics), RollupStoreRef($rollup), ArchiveStoreRef($runtime_ops))}/>
					</Router>
				</Route>
				<Stack>
					<DynamoTableBlock bx:ref="analytics" label="analytics" ttl="ttl"/>
					<DynamoTableBlock bx:ref="rollup" label="analytics-rollup"/>
					<S3BucketBlock bx:ref="runtime_ops" label="runtime-ops" deploy_versioned=false runtime_write=true object_versioning=true/>
					<LambdaJobBlock bx:ref="rollup_fn" label="rollup" exec_route="jobs" features="aws_sdk,lambda"/>
					<ScheduledJobBlock label="rollup-daily" {InvokeTarget($rollup_fn)} schedule="cron(0 3 * * ? *)" path="rollup"/>
				</Stack>
			</Fragment>"#,
		);
		// the expiring table and the aggregates that outlive it
		let mut tables = world
			.query::<&DynamoTableBlock>()
			.iter(&world)
			.map(|table| {
				(
					table.label().to_string(),
					table.ttl().clone().map(|attribute| attribute.to_string()),
				)
			})
			.collect::<Vec<_>>();
		tables.sort();
		tables.xpect_eq(vec![
			("analytics".to_string(), Some("ttl".to_string())),
			("analytics-rollup".to_string(), None),
		]);
		// nothing but the timer may reach a sweep that rewrites every row
		world
			.query::<&LambdaBlock>()
			.single(&world)
			.unwrap()
			.http()
			.xpect_false();

		// the job names all three stores by relation, and each one resolves to
		// the declaration whose name the deploy provisions
		let (job, events, rollups, archive) = world
			.query::<(Entity, &StoreRef, &RollupStoreRef, &ArchiveStoreRef)>()
			.single(&world)
			.map(|(job, events, rollups, archive)| {
				(job, events.0, rollups.0, archive.0)
			})
			.unwrap();
		world
			.entity(job)
			.contains::<AnalyticsRollupJob>()
			.xpect_true();
		// the job's `Router` is its own url space, rooted at `/`: the schedule's
		// `path="rollup"` resolves inside it, and the enclosing space — the one a
		// served site dispatches into — cannot reach it at any path.
		let jobs_router = world.entity(job).get::<ChildOf>().unwrap().parent();
		RouteTree::of(&world, jobs_router)
			.unwrap()
			.find(&["rollup"])
			.xpect_some();
		let outer = RouteTree::of(&world, router).unwrap();
		outer.find(&["rollup"]).xpect_none();
		outer.find(&["jobs", "rollup"]).xpect_none();
		let stack = Stack::default().resolve(&PackageConfig {
			app_name: "beet-site".into(),
			..default()
		});
		world
			.entity(events)
			.get::<DynamoTableBlock>()
			.unwrap()
			.table_name(&stack)
			.xpect_eq("beet-site--dev--analytics");
		world
			.entity(rollups)
			.get::<DynamoTableBlock>()
			.unwrap()
			.table_name(&stack)
			.xpect_eq("beet-site--dev--analytics-rollup");
		world
			.entity(archive)
			.get::<S3BucketBlock>()
			.unwrap()
			.label()
			.as_str()
			.xpect_eq("runtime-ops");

		// ..and the whole stack renders, which is where an unpointed schedule or
		// an unlowerable grant fails
		let root = world
			.query_filtered::<Entity, With<Stack>>()
			.single(&world)
			.unwrap();
		RenderScope::render(&mut world, root)
			.unwrap()
			.finish()
			.unwrap()
			.xmap(|(.., config)| config)
			.to_json_string()
			.unwrap()
			.as_str()
			.xpect_contains("aws_scheduler_schedule")
			.xpect_contains("cron(0 3 * * ? *)")
			// the archive bucket is writable by the runtime, the app bucket is not
			.xpect_contains("beet-site--dev--runtime-ops")
			.xpect_contains(
				r#""ttl":[{"attribute_name":"ttl","enabled":true}]"#,
			)
			// a job sweeps a store rather than answering a request, so it takes
			// the long timeout a served function has no use for
			.xpect_contains(r#""timeout":900"#)
			// an invoke-only function publishes nothing
			.xnot()
			.xpect_contains("aws_apigatewayv2");
	}

	/// The `shared`-stage stack, the shape the site entry declares: its verb
	/// routes nest under the `shared/` prefix, the assets bucket resolves the
	/// shared stack by ancestry (`beet-site--shared--assets`), and the app name
	/// comes from the app's own `<PackageConfig/>` rather than a prop.
	///
	/// The stage rides the `<Stack>` COMPONENT, registered in every native
	/// build, so a binary that linked no deploy templates still reads `shared`
	/// off the tree. A stage on a template prop is absent exactly when the
	/// template is, which is a lean binary quietly resolving the stage stack.
	#[beet_core::test]
	fn shared_stack_prefixes_verbs_and_names_bucket() {
		let mut world = test_world();
		world.insert_resource(PackageConfig {
			app_name: "beet-site".into(),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<Route path="shared">
				<Stack stage="shared">
					<S3BucketBlock label="assets" deploy_versioned=false public_read=true force_destroy=false/>
					<DeployRoutes/>
					<Route path="push" {ExchangeSequence}>
						<DirSync bucket="assets" local_dir="site/assets"/>
					</Route>
					<Route path="pull" {ExchangeSequence}>
						<DirSync bucket="assets" local_dir="site/assets" {SyncS3Bucket{direction:Pull, no_sign_request:true}}/>
					</Route>
				</Stack>
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

	/// A host template that OWNS its `Router` (the router is inside the bundle
	/// rather than an ancestor in the entry), the shape whose slot content
	/// reparents under that router only after slot resolution.
	#[template]
	pub fn SlottedHost() -> impl Bundle {
		(CliServer::default(), children![(
			Stack::default(),
			Router::with_defaults(),
			children![Validate, Plan, SlotTarget::new()],
		)])
	}

	/// A router-owning host's routes AND the ones an entry declares in its slot
	/// both land in the tree, which is the whole point of the slot: an entry
	/// gets the host's verbs for free and adds its own beside them.
	///
	/// REGRESSION: slot content registered its routes BEFORE the splice
	/// reparented it under the host's router, and they landed in the enclosing
	/// url space instead. Every slotted route in such an entry 404'd while
	/// `validate`/`plan` — declared in the same bundle, so never reparented —
	/// worked, which is what made it look like a problem with the entry rather
	/// than with the host. `SpawnTemplate` now wakes the route-tree rebuild.
	#[beet_core::test]
	fn host_mounts_slotted_routes() {
		let mut world = test_world();
		world.register_template::<SlottedHost>();
		world.insert_resource(PackageConfig {
			app_name: "beetmash".into(),
			..default()
		});
		let router = world.spawn(Router::with_defaults()).id();
		spawn_markup(
			&mut world,
			router,
			r#"<SlottedHost>
				<Route path="deploy" {ExchangeSequence}>
					<TofuApply/>
				</Route>
				<Route path="audit" {ExchangeSequence}>
					<TofuApply/>
				</Route>
			</SlottedHost>"#,
		);
		let host = world
			.query_filtered::<Entity, With<Stack>>()
			.single(&world)
			.unwrap();
		let tree = RouteTree::of(&world, host).unwrap();
		tree.find(&["validate"]).xpect_some();
		tree.find(&["deploy"]).xpect_some();
		tree.find(&["audit"]).xpect_some();
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
				app_name: "beet-site".into(),
				..default()
			});
			let router = world.spawn(Router::with_defaults()).id();
			spawn_markup(&mut world, router, markup);
			world
				.query::<&SyncS3Bucket>()
				.single(&world)
				.unwrap()
				.clone()
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
