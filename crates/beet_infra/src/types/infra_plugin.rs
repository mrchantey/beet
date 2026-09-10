use beet_core::prelude::*;

/// The infra runtime + the deploy block/action type registrations, so adding
/// `InfraPlugin` makes every compiled deploy type spawnable by tag (eg
/// `<CloudflareWorkerBlock/>`, `<TofuApply/>`) independent of the example
/// wiring. Each `register_type` is gated by the same feature as the type's
/// definition, so only the types actually compiled register.
///
/// The plugin itself is target-agnostic: the *definitions* (blocks, variables)
/// register everywhere, so a wasm consumer can author and serialize a stack, and
/// only the deploy actions — which shell out — are native-only.
#[derive(Default)]
pub struct InfraPlugin;

impl Plugin for InfraPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		#[cfg(feature = "deploy")]
		app.init_plugin::<beet_router::prelude::RouterPlugin>();

		// the identity every declaration composes its name from, registered in
		// every native build so `<Stack stage="shared"/>` authors anywhere, and
		// this launch's deploy mechanics beside it. `Deployment` is a resource
		// rather than derived per read, so one launch publishes every artifact
		// under one id.
		app.register_type::<crate::prelude::Stack>()
			.init_resource::<crate::prelude::Deployment>();

		// the deploy render schedule: every declaration lands before any render
		// reads the grant pool. Target-agnostic like the definitions, so a wasm
		// consumer renders the stack it cannot apply.
		use crate::prelude::DeployRender;
		use crate::prelude::DeployRenderSet;
		app.init_schedule(DeployRender);
		app.configure_sets(
			DeployRender,
			(DeployRenderSet::Declare, DeployRenderSet::Render).chain(),
		);

		// the deploy `Variable` + its value resolution, a field of the blocks'
		// `env_vars` (always compiled, in `types/`).
		app.register_type::<crate::types::Variable>()
			.register_type::<crate::types::VariableValue>();

		// the blocks a beet *application* declares (the bucket it is served from,
		// the stores it records to), so `<S3BucketBlock label="app"/>` and
		// `<DynamoTableBlock label="events"/>` spawn by tag in any build
		// carrying their default-on binding features. The site's own analytics
		// are blob-backed, so its stores are buckets; a workload wanting
		// indexed queries declares the table instead.
		#[cfg(feature = "bindings_aws_common")]
		app.register_type::<crate::prelude::S3BucketBlock>()
			.register_type::<crate::prelude::PrefixExpiry>()
			// ..and the compute's half of the entry-document reference, whose
			// agreement with the bucket's `deploy_versioned` is asserted in the
			// render rather than left to convention. See `RepoBucket`.
			.register_type::<crate::prelude::RepoBucket>()
			.add_systems(
				DeployRender,
				(
					crate::types::declare::<crate::prelude::S3BucketBlock>
						.in_set(DeployRenderSet::Declare),
					(
						crate::types::render::<crate::prelude::S3BucketBlock>,
						crate::blocks::assert_repo_buckets,
					)
						.in_set(DeployRenderSet::Render),
				),
			);
		#[cfg(feature = "bindings_aws_dynamo")]
		app.register_type::<crate::prelude::DynamoTableBlock>()
			.add_systems(
				DeployRender,
				(
					crate::types::declare::<crate::prelude::DynamoTableBlock>
						.in_set(DeployRenderSet::Declare),
					crate::types::render::<crate::prelude::DynamoTableBlock>
						.in_set(DeployRenderSet::Render),
				),
			);

		// ..and the runtime half of those declarations: one observer per block
		// type attaching the live store, so the deploy meaning (the render
		// systems above) and the runtime meaning hang off the one entity the
		// markup declared.
		#[cfg(all(
			feature = "bindings_aws_common",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_s3_store);
		#[cfg(all(
			feature = "bindings_aws_dynamo",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_table_store);

		// the serverless function, so a stack authors `<LambdaBlock
		// label="rollup"/>` from markup rather than only from Rust.
		#[cfg(feature = "lambda_block")]
		app.register_type::<crate::prelude::LambdaBlock>()
			.add_systems(
				DeployRender,
				(
					crate::types::declare::<crate::prelude::LambdaBlock>
						.in_set(DeployRenderSet::Declare),
					crate::prelude::LambdaBlock::render
						.in_set(DeployRenderSet::Render),
				),
			);

		// the compute and failover blocks are Rust-authored (no register_type
		// yet), but their render systems run wherever they compile.
		#[cfg(feature = "fargate_block")]
		app.add_systems(
			DeployRender,
			(
				crate::types::declare::<crate::prelude::FargateBlock>
					.in_set(DeployRenderSet::Declare),
				crate::prelude::FargateBlock::render
					.in_set(DeployRenderSet::Render),
			),
		);
		#[cfg(feature = "lightsail_block")]
		app.add_systems(
			DeployRender,
			(
				crate::types::declare::<crate::prelude::LightsailBlock>
					.in_set(DeployRenderSet::Declare),
				crate::prelude::LightsailBlock::render
					.in_set(DeployRenderSet::Render),
			),
		);
		#[cfg(feature = "cloudflare_dns")]
		app.add_systems(
			DeployRender,
			crate::types::render::<crate::prelude::CloudflareFailoverBlock>
				.in_set(DeployRenderSet::Render),
		);

		// the recurring timer and the relation naming the lambda it invokes, ie
		// `<ScheduledJobBlock {InvokeTarget($rollup)}
		// schedule="cron(0 3 * * ? *)" path="analytics/rollup"/>`.
		#[cfg(feature = "scheduled_job_block")]
		app.register_type::<crate::prelude::ScheduledJobBlock>()
			.register_type::<crate::prelude::InvokeTarget>()
			.register_type::<crate::prelude::Invokers>()
			.add_systems(
				DeployRender,
				crate::prelude::ScheduledJobBlock::render
					.in_set(DeployRenderSet::Render),
			);

		// the network and the database, spawned by tag (`<VpcBlock bx:ref="net"
		// label="net"/>`, `<RdsPostgresBlock label="db" {VpcRef($net)}/>`) in
		// any build carrying them, and the relations their consumers name them
		// through.
		#[cfg(feature = "vpc_block")]
		app.register_type::<crate::prelude::VpcBlock>()
			.register_type::<crate::prelude::VpcRef>()
			.register_type::<crate::prelude::VpcConsumers>()
			.register_type::<crate::prelude::SubnetTier>()
			.add_systems(
				DeployRender,
				crate::types::render::<crate::prelude::VpcBlock>
					.in_set(DeployRenderSet::Render),
			);
		#[cfg(feature = "rds_postgres_block")]
		app.register_type::<crate::prelude::RdsPostgresBlock>()
			.register_type::<crate::prelude::DatabaseRef>()
			.register_type::<crate::prelude::DatabaseConsumers>()
			.add_systems(
				DeployRender,
				(
					crate::types::declare::<crate::prelude::RdsPostgresBlock>
						.in_set(DeployRenderSet::Declare),
					crate::prelude::RdsPostgresBlock::render
						.in_set(DeployRenderSet::Render),
				),
			);

		// the parameter-store composition every generated credential is named
		// by, ie the `<EnsureSecret secret="db-password"/>` attribute.
		app.register_type::<crate::prelude::SecretRef>();

		// the zone a block publishes into, a field of every block that names a
		// hostname. Registered wherever the module compiles, since a block
		// authored by tag can only carry one if the type it holds resolves.
		#[cfg(any(
			feature = "lambda_block",
			feature = "fargate_block",
			feature = "lightsail_block",
			feature = "cloudflare_dns"
		))]
		app.register_type::<crate::prelude::DnsProvider>();

		// the mail stack, spawned by tag: the domain declaration
		// (`<MailDomainBlock domain="stalwart.beetmash.com"/>`), the box that
		// serves it, and the identity inputs both are authored from. Definitions,
		// so every target: a wasm consumer can author the stack it cannot deploy.
		//
		// Both mail blocks render bespoke, because both read the relay composed
		// beside the domains: the domain emits against it, and the box emits its
		// SES sending identity only if some domain resolves SES.
		#[cfg(feature = "mail")]
		app.add_systems(
			DeployRender,
			(
				crate::prelude::MailDomainBlock::declare
					.in_set(DeployRenderSet::Declare),
				(
					crate::prelude::MailDomainBlock::render,
					crate::prelude::StalwartBlock::render,
				)
					.in_set(DeployRenderSet::Render),
			),
		);
		// the relays a domain composes to say where its outbound mail leaves
		// through, ie `<MailDomainBlock domain=".." {SesRelay}/>`; absent both,
		// the box delivers directly.
		#[cfg(feature = "mail")]
		app.register_type::<crate::prelude::MailDomainBlock>()
			.register_type::<crate::prelude::MailRecords>()
			.register_type::<crate::prelude::SesRelay>()
			.register_type::<crate::prelude::ComailRelay>()
			.register_type::<crate::prelude::StalwartBlock>()
			.register_type::<crate::prelude::Member>()
			.register_type::<crate::prelude::Mailbox>()
			.register_type::<crate::prelude::Alias>()
			.register_type::<crate::prelude::MtaStsPolicy>()
			.register_type::<crate::prelude::MtaStsMode>();

		// the cloudflare deploy blocks, spawned by tag. Definitions, so every target.
		#[cfg(feature = "cloudflare_block")]
		app.register_type::<crate::prelude::CloudflareWorkerBlock>()
			.register_type::<crate::prelude::CloudflareContainerBlock>();

		// the cloudflare config components + the directly-spawnable cloudflare
		// deploy actions (`#[action]` + `#[reflect(Component,
		// Default)]`), all of which drive `wrangler` as a child process.
		#[cfg(all(feature = "cloudflare_block", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::CloudflareR2Sync>()
			.register_type::<crate::prelude::CloudflareBench>()
			.register_type::<crate::prelude::CloudflareWatch>()
			.register_type::<crate::prelude::CloudflareDestroy>();

		// the tofu apply action + its layer settings (`<TofuApply layer="storage"/>`),
		// and the zone edge setup/purge (the whole `actions` module is gated on
		// `deploy`, and is native-only).
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app
			// the step a content-only verb runs first, so it publishes into the
			// live version rather than minting one nothing serves.
			.register_type::<crate::prelude::AdoptCurrentDeploy>()
			.register_type::<crate::prelude::TofuApply>()
			// the two teardown steps: `tofu destroy`, and the state carriers it
			// leaves behind. They converge either side of the apply, so a destroy
			// group runs them in the opposite order to this list.
			.register_type::<crate::prelude::TofuDestroy>()
			.register_type::<crate::prelude::StackTeardown>()
			.register_type::<crate::prelude::CloudflareZoneSetup>()
			.register_type::<crate::prelude::CloudflarePurgeCache>();

		// the create-if-missing secret step, which every stack holding a
		// generated credential runs before its apply.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::EnsureSecret>();

		// the mail stack's deploy verbs: the sovereign signing key minted
		// before the apply that publishes it, the comail enrolment check that
		// hands over the records a human enrolled and the scheduled poll that
		// gives its alarms something to read, the pre-apply data snapshot,
		// reverse record, declarative apply into the mail server's own data
		// store, mta-sts policy host, end-to-end probe, two liveness checks,
		// restore drill and zone audit.
		#[cfg(all(
			feature = "deploy",
			feature = "mail",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::EnsureDkimKey>()
			.register_type::<crate::prelude::ComailEnroll>()
			.register_type::<crate::prelude::ComailDeliverability>()
			.register_type::<crate::prelude::EipReverseDns>()
			.register_type::<crate::prelude::EipReverseDnsReset>()
			.register_type::<crate::prelude::StalwartSnapshot>()
			.register_type::<crate::prelude::StalwartProvision>()
			.register_type::<crate::prelude::MtaStsPublish>()
			.register_type::<crate::prelude::MtaStsUnpublish>()
			.register_type::<crate::prelude::MailProbe>()
			.register_type::<crate::prelude::MailCredentials>()
			.register_type::<crate::prelude::MailHealth>()
			.register_type::<crate::prelude::MailRestoreDrill>()
			.register_type::<crate::prelude::ZoneAudit>()
			.register_type::<crate::prelude::AllowedRecord>()
			// the audit's scope selector, so `<ZoneAudit scope="Zone"/>`
			// resolves the variant rather than silently keeping the default.
			.register_type::<crate::prelude::ZoneAuditScope>()
			// the two paired declarations, each spawning its up-action and its
			// down-action from one tag at one file position, so the deploy and
			// teardown groups cannot drift apart.
			.register_template::<crate::prelude::EipReverseRecord>()
			.register_template::<crate::prelude::MtaStsPolicyHost>();

		// the bucket sync settings (`{SyncS3Bucket{delete:true}}`), the direction
		// enum a markup attribute names by variant, and the `<DirSync>` front-end
		// that binds a local dir to a bucket by label.
		#[cfg(all(
			feature = "deploy",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::SyncS3Bucket>()
			.register_type::<beet_net::prelude::SyncDirection>()
			.register_type::<crate::prelude::DirSync>()
			.add_observer(crate::actions::attach_dir_sync_store)
			// the retention window over the ledger's versions, which prunes
			// each version's binary and its document root together.
			.register_type::<crate::prelude::PruneVersions>();

		// the borrowed-paths copy (`<DirCopy src=".." dest=".." paths=".."/>`),
		// plain fs work so it needs no cloud backend.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::DirCopy>();

		// the CloudWatch tail and the target it composes its log group from.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::AwsWatch>()
			.register_type::<crate::prelude::WatchTarget>();

		// the full-lifecycle smoke-test action: reads a bucket's `BlobStore` (so
		// `aws_sdk`-gated like the store) and lives in the `actions` module (so
		// `deploy`-gated and native-only). Register it only when all three compile it.
		#[cfg(all(
			feature = "deploy",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LifecycleProbe>();

		// the two steps that make a running Lightsail box run what the stores
		// hold: a release onto the deploy's binary (the counterpart to its
		// machine-config-only user data), and a restart after a content sync.
		#[cfg(all(
			feature = "deploy",
			feature = "lightsail_block",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LightsailRelease>()
			.register_type::<crate::prelude::LightsailRestart>();

		// the docker/podman image build action + its engine selector. It lives in
		// the `actions` module, so it is `deploy`-gated and native-only like the
		// rest of them, on top of the `fargate_block` its own module is cut by.
		#[cfg(all(
			feature = "deploy",
			feature = "fargate_block",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::BuildDockerImage>()
			.register_type::<crate::prelude::ContainerEngine>();
	}
}

// native-only, matching the deploy/mail verbs the plugin registers: those types
// do not exist in a wasm build, so their tag tests cannot either.
#[cfg(all(
	test,
	feature = "mail",
	feature = "deploy",
	not(target_arch = "wasm32")
))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	#[cfg(feature = "scheduled_job_block")]
	use beet_net::prelude::*;

	/// The world an entry's markup builds into: the plugin under test plus the
	/// document machinery a `.bsx` load runs through.
	fn spawn(markup: &str) -> World {
		let mut world =
			(AsyncPlugin, TemplatePlugin, DocumentPlugin, InfraPlugin)
				.into_world();
		let nodes =
			BsxNode::parse_document(markup, &BsxParseConfig::bsx()).unwrap();
		world
			.spawn(())
			.insert_template(BsxTemplate::container(
				nodes,
				BsxTemplateRegistry::default(),
			))
			.unwrap();
		world.flush();
		world
	}

	/// The mail stack authors from markup, which is the whole reason its types
	/// reflect: an entry declares the domain and the box as tags, and the
	/// cross-block references are `bx:ref` relations resolving to the
	/// declaration entities (forwards or backwards, the resolver handles both).
	///
	/// REGRESSION: `MailDomainBlock` and `StalwartBlock` were not registered
	/// (they hold a `DnsProvider`, which was not `Reflect`), so both tags
	/// resolved to nothing and an entry declaring them built an empty stack
	/// that deployed successfully.
	#[beet_core::test]
	fn the_mail_blocks_spawn_by_tag() {
		let mut world = spawn(
			r#"<Fragment>
				<MailDomainBlock
					domain="news.beetmash.com"
					mail_host="mail.beetmash.com"
					report_domain="stalwart.beetmash.com"
					catch_all="publications"
					mailboxes={[{localpart:"publications"}]}
					aliases={[{localpart:"blog", target:"publications"}]}
					mta_sts={{mode:Enforce}}/>
				<StalwartBlock label="mail" hostname="mail.beetmash.com"
					blob_bucket="mail-blobs"
					ssh_public_key="ssh-ed25519 AAAA pete"
					{VpcRef($net)}/>
				<VpcBlock bx:ref="net" label="net"/>
			</Fragment>"#,
		);
		let domain = world.query::<&MailDomainBlock>().single(&world).unwrap();
		domain.domain().as_str().xpect_eq("news.beetmash.com");
		domain
			.report_domain()
			.as_str()
			.xpect_eq("stalwart.beetmash.com");
		domain.mailboxes().len().xpect_eq(1);
		domain.aliases()[0]
			.target()
			.as_str()
			.xpect_eq("publications");
		domain.mta_sts().mode().xpect_eq(MtaStsMode::Enforce);
		domain.validate().unwrap();

		let (mail_box, vpc_ref) = world
			.query::<(&StalwartBlock, &VpcRef)>()
			.single(&world)
			.unwrap();
		mail_box.validate().unwrap();
		// each relation resolves to the entity carrying the declared block
		world
			.entity(vpc_ref.0)
			.get::<VpcBlock>()
			.unwrap()
			.label()
			.as_str()
			.xpect_eq("net");
	}

	/// The relay is composed BESIDE the domain rather than named as a field, so
	/// it authors as a spread and resolves by ancestry: a stack-level one covers
	/// every domain under it, a domain's own overrides it, and neither is the
	/// default.
	///
	/// A relay component the binary did not register would resolve to nothing,
	/// and a stack whose author wrote `{SesRelay}` would deploy delivering
	/// directly: green, and quietly a different mail system.
	#[beet_core::test]
	fn the_relays_spawn_as_spreads_and_resolve_by_ancestry() {
		let mut world = spawn(
			r#"<Fragment {SesRelay{events_topic:"acme-ses-events"}}>
				<MailDomainBlock domain="stalwart.example.com" mail_host="mail.example.com"/>
				<MailDomainBlock domain="news.example.com" mail_host="mail.example.com"
					{ComailRelay}/>
			</Fragment>"#,
		);
		let mut domains = world
			.query::<(Entity, &MailDomainBlock)>()
			.iter(&world)
			.map(|(entity, block)| (entity, block.domain().clone()))
			.collect::<Vec<_>>();
		domains.sort_by_key(|(_, domain)| domain.clone());
		let resolved = world.with_state::<RelayQuery, _>(|relays| {
			domains
				.iter()
				.map(|(entity, domain)| {
					relays.resolve(*entity, domain).unwrap()
				})
				.collect::<Vec<_>>()
		});
		// the ancestor's, inherited by the domain that declares none
		resolved[1].xpect_eq(RelayMode::Ses(
			SesRelay::default().with_events_topic("acme-ses-events"),
		));
		// ..and the domain's own, which wins over it
		resolved[0].xpect_eq(RelayMode::Comail(ComailRelay::default()));
	}

	/// A field DERIVED from another at construction cannot survive being
	/// reflect-patched over the type's default, since the default derived from
	/// the empty value. Both of these read back as the declaration means them.
	///
	/// REGRESSION: `report_domain` was copied from `domain` in the constructor,
	/// so a markup-declared domain addressed its DMARC reports to `dmarc@` with
	/// no domain at all.
	#[beet_core::test]
	fn derived_fields_survive_a_markup_declaration() {
		let mut world = spawn(
			r#"<MailDomainBlock domain="stalwart.beetmash.com" mail_host="mail.beetmash.com"/>"#,
		);
		world
			.query::<&MailDomainBlock>()
			.single(&world)
			.unwrap()
			.dmarc_value()
			.xpect_contains("rua=mailto:dmarc@stalwart.beetmash.com");
	}

	/// REGRESSION: the database's master-password variable was stored as a
	/// field composed from the label, so a markup-declared database asked the
	/// apply for `var._password` while `EnsureSecret` supplied `db_password`.
	#[cfg(feature = "rds_postgres_block")]
	#[beet_core::test]
	fn a_markup_declared_database_derives_its_password_variable() {
		let mut world =
			spawn(r#"<RdsPostgresBlock label="db" database="mail"/>"#);
		world
			.query::<&RdsPostgresBlock>()
			.single(&world)
			.unwrap()
			.password()
			.key()
			.as_str()
			.xpect_eq("db_password");
	}

	/// The recurring timer and the lambda it drives author as tags, with the
	/// cross-block reference an `InvokeTarget` relation resolving to the
	/// lambda's declaration entity (a `bx:ref` may point forwards, the
	/// resolver handles it).
	///
	/// A block whose type does not register resolves to nothing at all, so an
	/// entry declaring a schedule would build a stack with no timer in it and
	/// deploy successfully. `LambdaBlock` was Rust-only until the schedule
	/// needed something to point at.
	#[cfg(feature = "scheduled_job_block")]
	#[beet_core::test]
	fn the_schedule_and_its_lambda_spawn_by_tag() {
		let mut world = spawn(
			r#"<Fragment>
				<ScheduledJobBlock label="rollup-daily" {InvokeTarget($rollup)}
					schedule="cron(0 3 * * ? *)" path="analytics/rollup"/>
				<LambdaBlock bx:ref="rollup" label="rollup"/>
			</Fragment>"#,
		);
		world
			.query::<&LambdaBlock>()
			.single(&world)
			.unwrap()
			.label()
			.as_str()
			.xpect_eq("rollup");
		let (schedule, target) = world
			.query::<(&ScheduledJobBlock, &InvokeTarget)>()
			.single(&world)
			.unwrap();
		schedule.path().as_str().xpect_eq("analytics/rollup");
		// the defaults a declaration does not name still hold
		schedule.method().xpect_eq(HttpMethod::Post);
		schedule.timezone().as_str().xpect_eq("UTC");
		schedule.validate().unwrap();
		// the relation resolves forwards to the entity carrying the lambda
		world
			.entity(target.0)
			.get::<LambdaBlock>()
			.unwrap()
			.label()
			.as_str()
			.xpect_eq("rollup");
	}

	/// The audit's scope is authored as its variant name, so an entry-level
	/// audit that means the whole zone says so rather than inheriting the
	/// stack-scoped default. A misspelling errors at build (the reflect scalar
	/// path rejects a string naming no unit variant), which is what keeps this
	/// from silently reading as a working declaration.
	#[beet_core::test]
	fn the_audit_scope_authors_by_variant_name() {
		let mut world = spawn(r#"<ZoneAudit scope="Zone"/>"#);
		world
			.query::<&ZoneAudit>()
			.single(&world)
			.unwrap()
			.scope
			.xpect_eq(ZoneAuditScope::Zone);
	}

	/// The post-apply verbs author as tags too, each naming what it works on
	/// rather than restating a composed name.
	#[beet_core::test]
	fn the_mail_verbs_spawn_by_tag() {
		let mut world = spawn(
			r#"<Fragment>
				<EnsureSecret secret="db-password" variable="db_password"/>
				<MailProbe mailbox="probe" sender_domain="news.beetmash.com"/>
				<ZoneAudit allowed={[{name:"beetmash.com", record_type:"MX", reason:"fastmail"}]}/>
			</Fragment>"#,
		);
		let secret = world.query::<&EnsureSecret>().single(&world).unwrap();
		secret.secret.label().as_str().xpect_eq("db-password");
		secret.variable.as_deref().unwrap().xpect_eq("db_password");
		world
			.query::<&MailProbe>()
			.single(&world)
			.unwrap()
			.sender_domain
			.as_str()
			.xpect_eq("news.beetmash.com");
		world.query::<&ZoneAudit>().single(&world).unwrap().allowed[0]
			.reason()
			.as_str()
			.xpect_eq("fastmail");
	}
}
