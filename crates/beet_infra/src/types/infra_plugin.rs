use beet_core::prelude::*;

/// The infra runtime + the deploy block/action type registrations, so adding
/// `InfraPlugin` makes every compiled deploy type spawnable by tag (eg
/// `<CloudflareWorkerBlock/>`, `<TofuApplyAction/>`) independent of the example
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

		// the deploy `Variable` + its value resolution, a field of the blocks'
		// `env_vars` (always compiled, in `types/`).
		app.register_type::<crate::types::Variable>()
			.register_type::<crate::types::VariableValue>();

		// the two blocks a beet *application* declares (the bucket it is served
		// from, the table it records to), so `<S3BucketBlock label="app"/>` and
		// `<DynamoTableBlock label="analytics"/>` spawn by tag in any build
		// carrying their default-on binding features.
		#[cfg(feature = "bindings_aws_common")]
		app.register_type::<crate::prelude::S3BucketBlock>();
		#[cfg(feature = "bindings_aws_dynamo")]
		app.register_type::<crate::prelude::DynamoTableBlock>();

		// ..and the runtime half of those declarations: one observer per block
		// type attaching the live store, so the deploy meaning (the always
		// compiled `ErasedBlock` hook) and the runtime meaning hang off the one
		// entity the markup declared.
		#[cfg(all(
			feature = "bindings_aws_common",
			feature = "aws_sdk",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_s3_store);
		#[cfg(all(
			feature = "bindings_aws_dynamo",
			not(target_arch = "wasm32")
		))]
		app.add_observer(crate::blocks::attach_table_store);

		// the network and the database, spawned by tag (`<VpcBlock label="net"/>`,
		// `<RdsPostgresBlock label="db" vpc="net"/>`) in any build carrying them.
		#[cfg(feature = "vpc_block")]
		app.register_type::<crate::prelude::VpcBlock>()
			.register_type::<crate::prelude::VpcRef>()
			.register_type::<crate::prelude::SubnetTier>()
			.register_type::<crate::prelude::SecurityGroupRef>();
		#[cfg(feature = "rds_postgres_block")]
		app.register_type::<crate::prelude::RdsPostgresBlock>()
			.register_type::<crate::prelude::DatabaseRef>();

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
		#[cfg(feature = "mail")]
		app.register_type::<crate::prelude::MailDomainBlock>()
			.register_type::<crate::prelude::MailRecords>()
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
		// deploy actions (`#[action(handler_only)]` + `#[reflect(Component,
		// Default)]`), all of which drive `wrangler` as a child process.
		#[cfg(all(feature = "cloudflare_block", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::CloudflareR2Sync>()
			.register_type::<crate::prelude::CloudflareBench>()
			.register_type::<crate::prelude::CloudflareWatch>()
			.register_type::<crate::prelude::CloudflareDestroy>()
			.register_type::<crate::prelude::CloudflareWorkerBuildAction>()
			.register_type::<crate::prelude::CloudflareWorkerDeployAction>()
			.register_type::<crate::prelude::CloudflareContainerDeployAction>();

		// the tofu apply action + its layer settings (`<TofuApply layer="storage"/>`),
		// and the zone edge setup/purge (the whole `actions` module is gated on
		// `deploy`, and is native-only).
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::TofuApplyAction>()
			.register_type::<crate::prelude::TofuApply>()
			.register_type::<crate::prelude::CloudflareZoneSetup>()
			.register_type::<crate::prelude::CloudflarePurgeCache>();

		// the create-if-missing secret step, which every stack holding a
		// generated credential runs before its apply.
		#[cfg(all(feature = "deploy", not(target_arch = "wasm32")))]
		app.register_type::<crate::prelude::EnsureSecret>()
			.register_type::<crate::prelude::EnsureSecretAction>();

		// the mail stack's post-apply verbs: the reverse record, the
		// declarative apply into the mail server's own data store, the
		// end-to-end probe and the zone audit.
		#[cfg(all(
			feature = "deploy",
			feature = "mail",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::EipReverseDns>()
			.register_type::<crate::prelude::EipReverseDnsAction>()
			.register_type::<crate::prelude::StalwartProvision>()
			.register_type::<crate::prelude::StalwartProvisionAction>()
			.register_type::<crate::prelude::MailProbe>()
			.register_type::<crate::prelude::MailProbeAction>()
			.register_type::<crate::prelude::ZoneAudit>()
			.register_type::<crate::prelude::ZoneAuditAction>()
			.register_type::<crate::prelude::AllowedRecord>();

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
			.add_observer(crate::actions::attach_dir_sync_store);

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

		// the post-apply release step: rolls a running Lightsail box onto the
		// deploy's binary, the counterpart to its machine-config-only user data.
		#[cfg(all(
			feature = "deploy",
			feature = "lightsail_block",
			not(target_arch = "wasm32")
		))]
		app.register_type::<crate::prelude::LightsailRelease>()
			.register_type::<crate::prelude::LightsailReleaseAction>();

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

#[cfg(all(test, feature = "mail", feature = "deploy"))]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

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
	/// cross-block references are the labels they are.
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
					vpc="net" database="db" blob_bucket="mail-blobs"
					ssh_public_key="ssh-ed25519 AAAA pete"/>
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

		let mail_box = world.query::<&StalwartBlock>().single(&world).unwrap();
		mail_box.vpc().label().as_str().xpect_eq("net");
		mail_box.database().label().as_str().xpect_eq("db");
		mail_box.security_group().label().as_str().xpect_eq("mail");
		mail_box.validate().unwrap();
	}

	/// A field DERIVED from another at construction cannot survive being
	/// reflect-patched over the type's default, since the default derived from
	/// the empty value. Both of these read back as the declaration means them.
	///
	/// REGRESSION: `report_domain` was copied from `domain` in the constructor,
	/// so a markup-declared domain addressed its DMARC reports to `dmarc@` with
	/// no domain at all; and the database's master-password variable was stored
	/// as a field composed from the label, so a markup-declared database asked
	/// the apply for `var._password` while `EnsureSecret` supplied `db_password`.
	#[beet_core::test]
	fn derived_fields_survive_a_markup_declaration() {
		let mut world = spawn(
			r#"<Fragment>
				<MailDomainBlock domain="stalwart.beetmash.com" mail_host="mail.beetmash.com"/>
				<RdsPostgresBlock label="db" vpc="net" database="mail"/>
			</Fragment>"#,
		);
		world
			.query::<&MailDomainBlock>()
			.single(&world)
			.unwrap()
			.dmarc_value()
			.xpect_contains("rua=mailto:dmarc@stalwart.beetmash.com");
		world
			.query::<&RdsPostgresBlock>()
			.single(&world)
			.unwrap()
			.password()
			.key()
			.as_str()
			.xpect_eq("db_password");
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
		secret.secret().label().as_str().xpect_eq("db-password");
		secret
			.variable()
			.clone()
			.unwrap()
			.as_str()
			.xpect_eq("db_password");
		world
			.query::<&MailProbe>()
			.single(&world)
			.unwrap()
			.sender_domain()
			.as_str()
			.xpect_eq("news.beetmash.com");
		world
			.query::<&ZoneAudit>()
			.single(&world)
			.unwrap()
			.allowed()[0]
			.reason()
			.as_str()
			.xpect_eq("fastmail");
	}
}
