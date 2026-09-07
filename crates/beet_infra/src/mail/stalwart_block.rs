use crate::bindings::*;
use crate::mail::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use heck::ToUpperCamelCase;
use serde_json::Value;
use serde_json::json;

/// The mail box: one EC2 instance running a pinned [Stalwart] release.
/// Mail metadata lives in Stalwart's embedded SQLite database on a persistent
/// data volume and message bodies live in its blob [`S3BucketBlock`], so the
/// instance remains cattle: any machine-config change replaces it, and inbound
/// SMTP during the rebuild is covered by sender retries.
///
/// The box is infrastructure, not a mail domain. Its `hostname` (the `A` record
/// this block emits, its rDNS, its certificate, its SMTP banner) stays put
/// while the [`MailDomainBlock`]s it serves come and go, which is what makes a
/// domain cutover a records change rather than a server move.
///
/// ## What is machine config and what is not
///
/// Stalwart `0.16` split its configuration in two, and this block follows the
/// split exactly. The on-disk `config.json` is the DATA store description alone
/// (the internally-tagged `DataStore` object); everything else — the blob,
/// search and in-memory stores, listeners, ACME, the SES relay route, the spam
/// filter, domains and accounts — lives *inside* the data store as JMAP objects
/// and is reconciled over the management API (`StalwartProvision`).
///
/// The file's absence is itself a state: a box with no `config.json` boots in
/// Stalwart's bootstrap mode, serving only the management endpoint, and the
/// server writes the file when provision claims the data store through the
/// `Bootstrap` singleton. So this block ships a `config.json.template` and
/// never the file itself: first boot finds no file and waits to be claimed, and
/// every later start copies the static SQLite template into place. Reconfiguring
/// a running server never touches this block at all.
///
/// ## The rebuild rule
///
/// Everything rendered into user_data is machine identity, and any change to it
/// replaces the instance (`user_data_replace_on_change`). Secrets are therefore
/// deliberately absent: until the store is claimed, the boot script fetches the
/// recovery-admin password from SSM through the instance profile; afterwards it
/// fetches nothing. The rendered script holds the parameter name only.
///
/// [Stalwart]: https://stalw.art
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct StalwartBlock {
	/// Label prefixing every terraform resource, including the box's own
	/// security group, ie the one its ingress admission names as its source.
	label: SmolStr,
	/// The box's fqdn, ie `mail.beetmash.com`: the `A` record, the rDNS target,
	/// the ACME certificate subject and the SMTP banner.
	hostname: SmolStr,

	/// The [`S3BucketBlock`] holding message bodies, by label. Declare it
	/// `with_runtime_write(true)` so the grant this block lowers can store.
	blob_bucket: SmolStr,
	/// The [`S3BucketBlock`] the nightly database dump is written to, by label,
	/// also `with_runtime_write(true)`. Empty installs no backup timer at all,
	/// which is a deliberate declaration rather than a default: mail metadata
	/// lives in one database, and a stack that keeps no copy of it should have
	/// said so.
	///
	/// The `archive` bucket in the taxonomy. A dump is a COPY, so the expiry
	/// belongs to its [`BACKUP_PREFIX`](Self::BACKUP_PREFIX) rather than to the
	/// bucket: the same archive holds prefixes whose contents are the only copy
	/// of what is in them and expire never.
	backup_bucket: SmolStr,
	/// The zone the box's `A` record is published into. Must be DNS-only: SMTP,
	/// IMAP and ACME TLS-ALPN-01 all need the origin reached directly, so a
	/// proxied record is a config-time error rather than a mystery outage.
	#[get(skip)]
	#[set_with(unwrap_option)]
	dns: Option<DnsProvider>,
	/// The stage that OWNS the shared names below, ie the one whose deploy may
	/// publish them.
	///
	/// A stack's resources are named `<app>--<stage>--<label>`, but a mail
	/// name is not: `mail.beetmash.com` and `stalwart.beetmash.com` are the
	/// real names of real mail, and a second stage deploying the same
	/// declaration would publish a SECOND record at each of them. Cloudflare
	/// accepts that, and receivers round-robin between the live box and
	/// whatever the other stage stood up, so an experiment takes production
	/// mail down without erroring anywhere.
	///
	/// So a stage that is not this one emits its SES identity and its
	/// infrastructure and touches no record at all. Empty means no guard,
	/// which is right for a stack whose domain nothing else serves.
	#[set_with(unwrap_option, into)]
	dns_stage: Option<SmolStr>,
	/// The SSH public key installed as the box's key pair. A public half only;
	/// the private key never touches this stack.
	ssh_public_key: SmolStr,
	instance_type: SmolStr,
	/// Root EBS volume size in GB, encrypted gp3.
	volume_gb: i64,
	/// Persistent data volume size in GB, encrypted gp3. The volume survives an
	/// instance replacement and mounts at [`DATA_MOUNT`](Self::DATA_MOUNT).
	data_volume_gb: i64,
	/// Whether tofu prevents destroying the persistent data volume and asks AWS
	/// for a final snapshot. Defaults to protected in production and disposable
	/// elsewhere, so the same declaration can run a restore drill in another
	/// stage. Set explicitly to override that data grade.
	#[get(skip)]
	#[set_with(unwrap_option)]
	data_volume_protected: Option<bool>,
}

impl Default for StalwartBlock {
	fn default() -> Self { Self::new("", "") }
}

impl Block for StalwartBlock {
	fn label(&self) -> &SmolStr { &self.label }
}

impl StalwartBlock {
	/// The pinned Stalwart release. Bump deliberately with a changelog read:
	/// minors have carried config migrations, and `0.16` replaced the entire
	/// configuration model.
	pub const STALWART_VERSION: &'static str = "0.16.19";
	/// sha256 of [`STALWART_TARBALL`](Self::STALWART_TARBALL), so the boot
	/// script refuses a tarball GitHub did not serve at pin time.
	pub const STALWART_SHA256: &'static str =
		"a783283996616ed28e23b9ca98b8934fbaf1e0e371fcdd47e238a3065a95c853";
	/// The release asset for the Graviton box: a single static `stalwart`
	/// binary.
	pub const STALWART_TARBALL: &'static str =
		"stalwart-aarch64-unknown-linux-musl.tar.gz";

	/// The open ports, and nothing else: management (8080 in bootstrap mode)
	/// stays closed and is reached over the SSH port when provision needs it.
	pub const OPEN_PORTS: &'static [(i64, &'static str)] = &[
		(22, "ssh"),
		(25, "smtp"),
		(443, "https"),
		(465, "submissions"),
		(587, "submission"),
		(993, "imaps"),
	];

	/// The smallest Graviton instance with enough memory for an MTA plus its
	/// spam classifier.
	pub const INSTANCE_TYPE: &'static str = "t4g.small";
	/// The persistent EBS volume's tag and terraform-label suffix.
	pub const DATA_VOLUME_KIND: &'static str = "data";
	/// The stable EC2 attachment name. Nitro exposes it through an NVMe by-id
	/// link containing the EBS volume id rather than at this path.
	pub const DATA_DEVICE_NAME: &'static str = "/dev/sdf";
	/// The persistent volume's mount point and Stalwart's working directory.
	pub const DATA_MOUNT: &'static str = "/var/lib/stalwart";
	/// The embedded SQLite database on the persistent data volume.
	pub const DATABASE_PATH: &'static str = "/var/lib/stalwart/stalwart.db";

	/// The SSM public parameter naming the current AL2023 arm64 AMI. Resolved
	/// per apply, so an AMI release replaces the box on the next deploy: the
	/// cattle answer, and the patched-kernel answer.
	pub const AMI_PARAMETER: &'static str =
		"/aws/service/ami-amazon-linux-latest/al2023-ami-kernel-default-arm64";

	/// The administrator username, which names two different credentials in
	/// sequence: the recovery admin the box's `stalwart.env` declares while the
	/// data store is unclaimed, and the real `admin@<domain>` account the
	/// server creates when it is claimed. The recovery credential exists only
	/// for the window between them, and the boot renderer stops writing it the
	/// moment a `config.json` exists.
	pub const ADMIN_USER: &'static str = "admin";


	/// When the nightly dump runs, in UTC (`systemd` calendar syntax). Late
	/// evening in Sydney, ie the quietest hour for the mailboxes this serves.
	pub const BACKUP_SCHEDULE: &'static str = "*-*-* 14:30:00";

	/// The prefix every database snapshot is written under, so the bucket's
	/// lifecycle rule and an off-site `rclone` pull both have one path to name.
	pub const BACKUP_PREFIX: &'static str = "sqlite";

	pub fn new(
		label: impl Into<SmolStr>,
		hostname: impl Into<SmolStr>,
	) -> Self {
		Self {
			label: label.into(),
			hostname: hostname.into(),

			blob_bucket: SmolStr::default(),
			backup_bucket: SmolStr::default(),
			dns: None,
			dns_stage: None,
			ssh_public_key: SmolStr::default(),
			instance_type: Self::INSTANCE_TYPE.into(),
			volume_gb: 30,
			data_volume_gb: 20,
			data_volume_protected: None,
		}
	}

	/// The zone the box's `A` record is published into: the declared
	/// [`dns`](Self::with_dns) provider, else a Cloudflare zone read from
	/// `CLOUDFLARE_ZONE_ID`, at this box's own hostname.
	///
	/// The hostname is the block's, never the provider's: a declaration names
	/// the box, and where the zone lives is a property of the launch.
	pub fn resolved_dns(&self) -> Option<DnsProvider> {
		if let Some(dns) = &self.dns {
			return Some(dns.clone());
		}
		#[cfg(feature = "cloudflare_dns")]
		return DnsProvider::cloudflare_env(self.hostname.clone());
		#[cfg(not(feature = "cloudflare_dns"))]
		None
	}

	/// Whether this stage may publish the box's name, ie whether it is the
	/// [`dns_stage`](Self::dns_stage) (or none was declared).
	///
	/// A stage that is not gets no address record, and therefore no
	/// certificate, so its deploy fails at provision — which is the right place
	/// for it to fail, rather than at the point where a second `A` record has
	/// already sent half the world's mail to the wrong box.
	pub fn owns_names(&self, stack: &ResolvedStack) -> bool {
		self.dns_stage
			.as_ref()
			.is_none_or(|owner| owner == stack.stage())
	}

	/// The label suffix of this box's security group, ie `mail--sg`.
	pub const SECURITY_GROUP: &'static str = "sg";

	/// The terraform ident of the security group this block declares.
	pub fn security_group_ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(format!(
			"{}--{}",
			self.label,
			Self::SECURITY_GROUP
		))
	}

	/// An interpolated reference to the box's security group id.
	pub fn security_group_id(&self, stack: &ResolvedStack) -> String {
		format!(
			"${{aws_security_group.{}.id}}",
			self.security_group_ident(stack).label()
		)
	}

	/// The CloudWatch log group the box's agent forwards `stalwart.log` to,
	/// shared with `WatchTarget::Instance` so `watch` tails the same group.
	pub fn log_group(&self, stack: &ResolvedStack) -> String {
		format!("/{}/{}/{}", stack.app_name(), self.label, stack.stage())
	}

	/// One of this box's secrets, under the stack's secret prefix, ie
	/// `/beetmash/prod/mail-admin-password`.
	fn secret(&self, suffix: &str) -> SecretRef {
		SecretRef::new(format!("{}-{suffix}", self.label))
	}


	/// Where `EnsureSecret` puts the bootstrap admin password and the boot
	/// script reads it back.
	pub fn admin_secret(&self) -> SecretRef { self.secret("admin-password") }

	/// The full parameter name of [`admin_secret`](Self::admin_secret).
	pub fn admin_secret_name(&self, stack: &ResolvedStack) -> String {
		self.admin_secret().name(stack)
	}

	/// Where terraform puts the SES SMTP username (the sending user's access
	/// key id). Read by `StalwartProvision` when it writes the SES relay route,
	/// not by the box.
	///
	/// The SES arm's credential is terraform-derived and so lives on the box's
	/// own parameter prefix; a comail credential is minted at enrolment and
	/// parked per DOMAIN ([`ComailRelay::did_secret`]), because its key pins the
	/// one domain it may send from.
	pub fn ses_smtp_user_secret(&self) -> SecretRef {
		self.secret("ses-smtp-user")
	}

	/// The full parameter name of [`ses_smtp_user_secret`](Self::ses_smtp_user_secret).
	pub fn ses_smtp_user_secret_name(&self, stack: &ResolvedStack) -> String {
		self.ses_smtp_user_secret().name(stack)
	}

	/// Where terraform puts the SES SMTP password, derived from the access key
	/// (`ses_smtp_password_v4`) so it exists nowhere else.
	pub fn ses_smtp_password_secret(&self) -> SecretRef {
		self.secret("ses-smtp-password")
	}

	/// The full parameter name of
	/// [`ses_smtp_password_secret`](Self::ses_smtp_password_secret).
	pub fn ses_smtp_password_secret_name(
		&self,
		stack: &ResolvedStack,
	) -> String {
		self.ses_smtp_password_secret().name(stack)
	}

	fn build_label(&self, suffix: &str) -> String {
		format!("{}--{suffix}", self.label)
	}

	/// The `Name` tag identifying this box's persistent data volume. Snapshot
	/// actions combine it with the stack's `Project` and `Stage` tags, so they
	/// find the declaration without reading tofu state.
	pub(crate) fn data_volume_tag_name(&self) -> String {
		self.build_label(Self::DATA_VOLUME_KIND)
	}

	fn data_volume_ident(&self, stack: &ResolvedStack) -> terra::Ident {
		stack.resource_ident(self.data_volume_tag_name())
	}

	fn data_volume_id(&self, stack: &ResolvedStack) -> String {
		format!(
			"${{aws_ebs_volume.{}.id}}",
			self.data_volume_ident(stack).label()
		)
	}

	fn tags(
		&self,
		stack: &ResolvedStack,
		kind: &str,
	) -> std::collections::BTreeMap<SmolStr, SmolStr> {
		[
			(
				SmolStr::from("Name"),
				self.build_label(kind).as_str().into(),
			),
			(SmolStr::from("Project"), stack.app_name().clone()),
			(SmolStr::from("Stage"), stack.stage().clone()),
		]
		.into_iter()
		.collect()
	}

	/// Whether the persistent data volume is protected from stack destruction.
	/// Production is durable and every other stage is disposable by default; an
	/// explicit declaration overrides the stage-derived data grade.
	pub fn data_volume_protected(&self, stack: &ResolvedStack) -> bool {
		self.data_volume_protected
			.unwrap_or_else(|| stack.is_production())
	}

	/// Rejects a declaration that cannot serve mail at config time.
	///
	/// A box with no network fails at render, where its [`VpcRef`] resolves.
	pub fn validate(&self) -> Result {
		if self.blob_bucket.is_empty() {
			bevybail!(
				"mail box '{}' names no blob bucket: `with_blob_bucket` the S3BucketBlock holding message bodies",
				self.label
			);
		}
		if self.ssh_public_key.is_empty() {
			bevybail!(
				"mail box '{}' has no ssh public key: port 22 is keypair-only, so a box without one is unreachable",
				self.label
			);
		}
		for label in self.hostname.split('.') {
			DnsProvider::validate_label(label, "mail box hostname")?;
		}
		#[cfg(feature = "cloudflare_dns")]
		if let Some(DnsProvider::Cloudflare { proxied: true, .. }) = &self.dns {
			bevybail!(
				"mail box '{}' must not be Cloudflare-proxied: SMTP, IMAP and ACME TLS-ALPN-01 all dial the origin directly",
				self.label
			);
		}
		Ok(())
	}
}

/// The [`DeployRender`] systems, registered by [`InfraPlugin`] beside the
/// type registration.
impl StalwartBlock {
	/// Renders the box and its secondaries into the config, resolving the
	/// [`VpcRef`] relation, reaching up to the mail domains declared beside it,
	/// and lowering the grants the stack's declarations contributed.
	///
	/// The domains are a cross-entity input like the relations are: the SES
	/// sending identity exists exactly when SOME domain relays through SES, and
	/// the box cannot see that from its own declaration.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		blocks: Query<(Entity, &StalwartBlock, Option<&VpcRef>)>,
		vpcs: Query<&VpcBlock>,
		domains: Query<(Entity, &MailDomainBlock)>,
		relays: RelayQuery,
	) {
		for (entity, block, vpc_ref) in blocks.iter() {
			let Ok(root) = scopes.get_entity(entity) else {
				continue;
			};
			// every domain rendering into the same scope, ie the box's own
			// stack and no other
			let relayed = domains
				.iter()
				.filter(|(domain, _)| {
					scopes.get_entity(*domain).is_ok_and(|it| it == root)
				})
				.map(|(domain, block)| {
					relays
						.resolve(domain, block.domain())
						.map(|relay| (block.domain().clone(), relay))
				})
				.collect::<Result<Vec<_>>>();
			let vpc = crate::types::related(
				&scopes,
				entity,
				&vpcs,
				vpc_ref.map(|vpc_ref| vpc_ref.0),
				"VpcRef",
				block.label(),
			);
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			match (vpc, relayed) {
				(Ok(vpc), Ok(relayed)) => {
					let relays = relayed.into_iter().fold(
						RelayModes::default(),
						|mut relays, (domain, relay)| {
							relays.insert(domain, relay);
							relays
						},
					);
					let access = scope.access();
					let (stack, _deployment, config) = scope.ctx();
					if let Err(err) =
						block.emit(stack, vpc, &access, &relays, config)
					{
						scope.error(err);
					}
				}
				(vpc, relayed) => {
					for err in [vpc.err(), relayed.err()].into_iter().flatten()
					{
						scope.error(err);
					}
				}
			}
		}
	}

	/// Emit this box's resources: the security group, the instance role
	/// lowered from the declared grants, the SES sending identity if any domain
	/// asks for one, and the instance itself.
	fn emit(
		&self,
		stack: &ResolvedStack,
		vpc: &VpcBlock,
		access: &AccessGrants,
		relays: &RelayModes,
		config: &mut terra::Config,
	) -> Result {
		self.validate()?;
		let group = self.emit_security_group(stack, config, vpc)?;
		let role = self.emit_instance_role(stack, config, access)?;
		// an IAM user holding a static credential is the one thing in this
		// stack that exists only because a provider's protocol demands it, so
		// it exists only while a domain actually relays through that provider.
		if relays.any_ses() {
			self.emit_ses_sender(stack, config)?;
		}
		self.emit_instance(stack, config, vpc, &group, &role)?;
		Ok(())
	}
}

impl StalwartBlock {
	/// Emits the box's security group: the mail port list in and everything out.
	///
	/// An MTA dials the world: peer MTAs on 25, relays on 587, S3, SSM, ACME,
	/// and GitHub.
	fn emit_security_group(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &VpcBlock,
	) -> Result<ResourceDef<AwsSecurityGroupDetails>> {
		let group = ResourceDef::new_primary(
			self.security_group_ident(stack),
			AwsSecurityGroupDetails {
				description: Some(
					format!("Mail box ports for {}", self.label).into(),
				),
				vpc_id: Some(vpc.id(stack).into()),
				tags: Some(self.tags(stack, Self::SECURITY_GROUP)),
				..default()
			},
		);
		config.add_resource(&group)?;
		for (port, service) in Self::OPEN_PORTS {
			config.add_resource(&ResourceDef::new_secondary(
				stack
					.resource_ident(self.build_label(&format!("in-{service}"))),
				AwsSecurityGroupRuleDetails {
					security_group_id: group.field_ref("id").into(),
					r#type: "ingress".into(),
					from_port: *port,
					to_port: *port,
					protocol: "tcp".into(),
					cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
					ipv6_cidr_blocks: Some(vec!["::/0".into()]),
					description: Some(SmolStr::from(*service)),
					..default()
				},
			))?;
		}
		config.add_resource(&ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("out-all")),
			AwsSecurityGroupRuleDetails {
				security_group_id: group.field_ref("id").into(),
				r#type: "egress".into(),
				from_port: 0,
				to_port: 0,
				protocol: "-1".into(),
				cidr_blocks: Some(vec!["0.0.0.0/0".into()]),
				ipv6_cidr_blocks: Some(vec!["::/0".into()]),
				description: Some("an MTA dials the world".into()),
				..default()
			},
		))?;

		group.xok()
	}

	/// The instance role: what a compromised box can do, in full. Lowered from
	/// the [`AccessGrants`] the stack's blocks declared, plus the two grants
	/// nothing declares because this block owns them (its bootstrap-admin
	/// parameter and log group).
	///
	/// This is the improvement over [`LightsailBlock`]'s static key: EC2
	/// carries a role through the instance profile, so no long-lived credential
	/// exists at all and IMDSv2 is the only place short-lived ones come from.
	fn emit_instance_role(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		access: &AccessGrants,
	) -> Result<ResourceDef<AwsIamInstanceProfileDetails>> {
		let role = ResourceDef::new_primary(
			stack.resource_ident(self.build_label("role")),
			AwsIamRoleDetails {
				assume_role_policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Effect": "Allow",
						"Principal": { "Service": "ec2.amazonaws.com" },
						"Action": "sts:AssumeRole"
					}]
				})
				.to_string()
				.into(),
				..default()
			},
		);
		let policy_ident = stack.resource_ident(self.build_label("policy"));
		let policy = ResourceDef::new_secondary(
			policy_ident.clone(),
			AwsIamRolePolicyDetails {
				name: Some(policy_ident.primary_identifier().clone()),
				role: role.field_ref("name").into(),
				policy: self.runtime_policy(stack, access)?.render().into(),
				..default()
			},
		);
		let profile_ident = stack.resource_ident(self.build_label("profile"));
		let profile = ResourceDef::new_secondary(
			profile_ident.clone(),
			AwsIamInstanceProfileDetails {
				name: Some(profile_ident.primary_identifier().clone()),
				role: Some(role.field_ref("name").into()),
				..default()
			},
		);
		config
			.add_resource(&role)?
			.add_resource(&policy)?
			.add_resource(&profile)?;
		profile.xok()
	}

	/// The inline policy document, lowered through the shared [`IamPolicy`]
	/// core, seeded with the two statements this block owns: its bootstrap-admin
	/// parameter and its CloudWatch log group. The blob store multiparts, so the
	/// write statement carries `s3:AbortMultipartUpload` through the
	/// per-compute knob.
	///
	/// SecureString decryption needs no `kms:` statement here: the AWS-managed
	/// `aws/ssm` key authorises account principals through its own key policy
	/// for requests made via SSM.
	fn runtime_policy(
		&self,
		stack: &ResolvedStack,
		access: &AccessGrants,
	) -> Result<IamPolicy> {
		let region = stack.region();
		let log_group = self.log_group(stack);
		IamPolicy::new(region.clone(), "stalwart box")
			.statement(json!({
				"Sid": "BootstrapAdmin",
				"Effect": "Allow",
				"Action": ["ssm:GetParameter"],
				"Resource": format!(
					"arn:aws:ssm:{region}:*:parameter{}",
					self.admin_secret_name(stack)
				)
			}))
			.write_action("s3:AbortMultipartUpload")
			.lower(access)?
			.statement(json!({
				"Sid": "OwnLogGroup",
				"Effect": "Allow",
				"Action": [
					"logs:CreateLogStream",
					"logs:PutLogEvents",
					"logs:DescribeLogStreams"
				],
				"Resource": [
					format!("arn:aws:logs:{region}:*:log-group:{log_group}"),
					format!("arn:aws:logs:{region}:*:log-group:{log_group}:*")
				]
			}))
			.xok()
	}

	/// The SES sending identity: an IAM user whose only permission is
	/// `ses:SendRawEmail`, whose access key IS the SMTP credential
	/// (`ses_smtp_password_v4` derives the password from the secret), parked in
	/// SSM for `StalwartProvision` to write into the relay route.
	///
	/// A user rather than the instance role because SES SMTP authentication is
	/// a static credential by protocol; scoping it to one action on one service
	/// is what bounds the loss if it leaks. The parameter values are terraform
	/// interpolations, so the secret transits state and nothing else, which is
	/// what the stack's state encryption is for.
	fn emit_ses_sender(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		let user = ResourceDef::new_primary(
			stack.resource_ident(self.build_label("ses-user")),
			AwsIamUserDetails::default(),
		);
		let policy_ident = stack.resource_ident(self.build_label("ses-policy"));
		let policy = ResourceDef::new_secondary(
			policy_ident.clone(),
			AwsIamUserPolicyDetails {
				name: Some(policy_ident.primary_identifier().clone()),
				user: user.field_ref("name").into(),
				// every identity in the account: identities are per-domain
				// blocks and this user sends for all of them.
				policy: json!({
					"Version": "2012-10-17",
					"Statement": [{
						"Sid": "SesSmtpRelay",
						"Effect": "Allow",
						"Action": "ses:SendRawEmail",
						"Resource": "*"
					}]
				})
				.to_string()
				.into(),
				..default()
			},
		);
		let key = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("ses-key")),
			AwsIamAccessKeyDetails {
				user: user.field_ref("name").into(),
				..default()
			},
		);
		let user_param = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("ses-smtp-user")),
			AwsSsmParameterDetails {
				name: self.ses_smtp_user_secret_name(stack).into(),
				r#type: "SecureString".into(),
				value: Some(key.field_ref("id").into()),
				..default()
			},
		);
		let password_param = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("ses-smtp-password")),
			AwsSsmParameterDetails {
				name: self.ses_smtp_password_secret_name(stack).into(),
				r#type: "SecureString".into(),
				value: Some(key.field_ref("ses_smtp_password_v4").into()),
				..default()
			},
		);
		config
			.add_resource(&user)?
			.add_resource(&policy)?
			.add_resource(&key)?
			.add_resource(&user_param)?
			.add_resource(&password_param)?;
		Ok(())
	}

	/// The encrypted gp3 data volume. It lives in the same availability zone as
	/// the box's subnet and is independent of the replaceable instance.
	fn data_volume(
		&self,
		stack: &ResolvedStack,
		vpc: &VpcBlock,
	) -> ResourceDef<AwsEbsVolumeDetails> {
		ResourceDef::new_secondary(
			self.data_volume_ident(stack),
			AwsEbsVolumeDetails {
				// the SUBNET's zone, by reference: a volume in a zone the box
				// is not in is an attachment that fails at apply
				availability_zone: vpc
					.subnet_availability_zone(stack, SubnetTier::Public, "a")
					.into(),
				encrypted: Some(true),
				final_snapshot: Some(self.data_volume_protected(stack)),
				size: Some(self.data_volume_gb),
				r#type: Some("gp3".into()),
				tags: Some(self.tags(stack, Self::DATA_VOLUME_KIND)),
				..default()
			},
		)
	}

	/// The replaceable edge between the persistent volume and one generation of
	/// the box. Stopping before detach flushes the old machine's filesystem.
	fn data_volume_attachment(
		&self,
		stack: &ResolvedStack,
		volume: &ResourceDef<AwsEbsVolumeDetails>,
		instance: &ResourceDef<AwsInstanceDetails>,
	) -> ResourceDef<AwsVolumeAttachmentDetails> {
		ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("data-attachment")),
			AwsVolumeAttachmentDetails {
				device_name: Self::DATA_DEVICE_NAME.into(),
				instance_id: instance.field_ref("id").into(),
				stop_instance_before_detaching: Some(true),
				volume_id: volume.field_ref("id").into(),
				..default()
			},
		)
	}

	/// The box itself: AMI resolved from the public AL2023 arm64 pointer, key
	/// pair, encrypted root, IMDSv2 required, user_data as machine identity,
	/// an EIP so the address (and its rDNS) survives every rebuild, the log
	/// group its agent forwards to, and the `A` record naming it.
	fn emit_instance(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		vpc: &VpcBlock,
		group: &ResourceDef<AwsSecurityGroupDetails>,
		profile: &ResourceDef<AwsIamInstanceProfileDetails>,
	) -> Result {
		let ami_label = stack
			.resource_ident(self.build_label("ami"))
			.label()
			.to_string();
		config.add_untyped_data_source(
			"aws_ssm_parameter",
			&ami_label,
			&json!({ "name": Self::AMI_PARAMETER }),
		)?;

		// `key_name_prefix` so a rotated public key is a NEW key pair name,
		// which forces instance replacement: EC2 only installs the key at
		// launch, so an in-place key update would be silently ignored.
		let keypair_ident = stack.resource_ident(self.build_label("keypair"));
		let keypair = ResourceDef::new_secondary(
			keypair_ident.clone(),
			AwsKeyPairDetails {
				key_name_prefix: Some(
					keypair_ident.primary_identifier().clone(),
				),
				public_key: self.ssh_public_key.clone(),
				..default()
			},
		);

		// declared so `tofu destroy` removes it; the agent writes into the
		// existing group rather than auto-creating an unmanaged one.
		let log_group = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("logs")),
			AwsCloudwatchLogGroupDetails {
				name: Some(self.log_group(stack).into()),
				retention_in_days: Some(30),
				..default()
			},
		);

		let data_volume = self.data_volume(stack, vpc);
		let user_data = self.build_user_data(stack)?;
		let instance_ident = stack.resource_ident(self.build_label("instance"));
		let instance = ResourceDef::new_secondary(
			instance_ident.clone(),
			AwsInstanceDetails {
				ami: Some(
					format!("${{data.aws_ssm_parameter.{ami_label}.value}}")
						.into(),
				),
				instance_type: Some(self.instance_type.clone()),
				subnet_id: Some(
					vpc.subnet_id(stack, SubnetTier::Public, "a").into(),
				),
				vpc_security_group_ids: Some(vec![
					group.field_ref("id").into(),
				]),
				key_name: Some(keypair.field_ref("key_name").into()),
				iam_instance_profile: Some(profile.field_ref("name").into()),
				user_data: Some(user_data),
				// the rebuild rule: an edited machine is a new machine.
				user_data_replace_on_change: Some(true),
				metadata_options: Some(vec![
					AwsInstanceResourceBlockTypeMetadataOptions {
						http_endpoint: Some("enabled".into()),
						// IMDSv2 only: an SSRF against a mail-adjacent web
						// surface must not read the role's credentials.
						http_tokens: Some("required".into()),
						..default()
					},
				]),
				root_block_device: Some(vec![
					AwsInstanceResourceBlockTypeRootBlockDevice {
						volume_size: Some(self.volume_gb),
						volume_type: Some("gp3".into()),
						encrypted: Some(true),
						..default()
					},
				]),
				tags: Some(self.tags(stack, "instance")),
				..default()
			},
		);

		// an EIP association fails while the vpc has no attached gateway, and
		// terraform cannot see that dependency through the label reference.
		let eip_ident = stack.resource_ident(self.build_label("eip"));
		let eip = ResourceDef::new_secondary(eip_ident, AwsEipDetails {
			domain: Some("vpc".into()),
			tags: Some(self.tags(stack, "eip")),
			depends_on: Some(vec![
				format!(
					"aws_internet_gateway.{}",
					vpc.ident(stack, VpcBlock::GATEWAY).label()
				)
				.into(),
			]),
			..default()
		});
		let association = ResourceDef::new_secondary(
			stack.resource_ident(self.build_label("eip-assoc")),
			AwsEipAssociationDetails {
				allocation_id: Some(eip.field_ref("id").into()),
				instance_id: Some(instance.field_ref("id").into()),
				..default()
			},
		);
		let data_attachment =
			self.data_volume_attachment(stack, &data_volume, &instance);

		config
			.add_resource(&keypair)?
			.add_resource(&log_group)?
			.add_resource(&data_volume)?
			.add_resource(&instance)?
			.add_resource(&data_attachment)?
			.add_resource(&eip)?
			.add_resource(&association)?;
		if self.data_volume_protected(stack) {
			config.set_lifecycle(
				"aws_ebs_volume",
				data_volume.ident().label(),
				json!({ "prevent_destroy": true }),
			)?;
		}
		config.set_lifecycle(
			"aws_volume_attachment",
			data_attachment.ident().label(),
			json!({
				"replace_triggered_by": [instance.field("id")]
			}),
		)?;

		// without an address record the hostname resolves nowhere, so the
		// certificate never issues, the `MX` targets nothing and the reverse
		// record is refused. Every one of those surfaces long after a green
		// apply, so the emit fails instead of quietly skipping. Not in
		// `validate`, since which zone a launch has is not a property of the
		// declaration.
		if !self.owns_names(stack) {
			warn!(
				"stage '{}' does not own '{}', so no address record is \
				published for it",
				stack.stage(),
				self.hostname
			);
			return Ok(());
		}
		let Some(dns) = self.resolved_dns() else {
			bevybail!(
				"mail box '{}' resolves no zone to publish '{}' into: set CLOUDFLARE_ZONE_ID or `with_dns` a provider",
				self.label,
				self.hostname
			);
		};
		dns.emit_address(
			stack,
			config,
			&self.build_label("dns"),
			&eip.field_ref("public_ip"),
			false,
		)?;

		config
			.add_output(format!("{}_public_ip", self.label), terra::Output {
				value: eip.field_ref("public_ip").into(),
				description: Some("The mail box public address".into()),
				sensitive: None,
			})?
			.add_output(
				format!("{}_eip_allocation", self.label),
				terra::Output {
					value: eip.field_ref("id").into(),
					description: Some(
						"The EIP allocation id, ie what EipReverseDns sets the PTR on"
							.into(),
					),
					sensitive: None,
				},
			)?
			.add_output(format!("{}_instance_id", self.label), terra::Output {
				value: instance.field_ref("id").into(),
				description: Some("The mail box instance id".into()),
				sensitive: None,
			})?;
		Ok(())
	}
}

/// The machine config renderers. Everything here lands in user_data, so
/// everything here obeys the rebuild rule; see the type docs.
impl StalwartBlock {
	/// Returns the on-disk `config.json` template.
	///
	/// Stalwart `0.16` expects the [`DataStore`](https://stalw.art/docs/storage/overview)
	/// object alone. The blob store is a registry object written by the
	/// `Bootstrap` claim, while search and in-memory stores use the data store via
	/// `Default`.
	pub(crate) fn data_store_config(&self) -> Value {
		json!({
			"@type": "Sqlite",
			"path": Self::DATABASE_PATH
		})
	}

	/// The blob store as the `Bootstrap` claim declares it, composed here
	/// beside the data store template so the box's file and the provision
	/// payload cannot drift.
	pub fn blob_store_config(&self, stack: &ResolvedStack) -> Value {
		let none = json!({ "@type": "None" });
		json!({
			"@type": "S3",
			"bucket": stack.resource_name(self.blob_bucket.clone()),
			// no static credential anywhere on this box: the S3 client
			// falls through its chain to the instance profile.
			"accessKey": none,
			"secretKey": none,
			"securityToken": none,
			"sessionToken": none,
			"region": {
				"@type": stack.region().to_upper_camel_case()
			}
		})
	}

	/// Returns the boot script at `/usr/local/bin/stalwart-secrets`.
	///
	/// Before every service start it renders the bootstrap admin credential into
	/// `stalwart.env` while the server still needs one, or copies the static
	/// SQLite data-store template into `config.json` after the store is claimed.
	///
	/// The `config.json` existence guard is the commissioning protocol, not
	/// caution: a box with NO file boots in Stalwart's bootstrap mode waiting
	/// for provision's `Bootstrap` claim, and the server writes the first file
	/// itself. Rendering one here on first boot would skip bootstrap mode
	/// entirely and boot a server with an unclaimed, tableless data store.
	///
	/// The SAME guard retires the recovery credential. `STALWART_RECOVERY_ADMIN`
	/// is a password that provisions the whole server and answers on a
	/// plaintext port, and it exists for one reason: to authenticate the claim
	/// on a server that has no accounts yet. The moment a `config.json` exists
	/// there IS an administrator account, so the env file is rewritten without
	/// it and the backdoor dies with the commissioning phase.
	///
	/// `render-store` forces the store render on a box whose disk is fresh but
	/// whose data store is already claimed — a rebuilt instance, where the file
	/// is missing and bootstrap mode is a lie. [`StalwartProvision`] is the one
	/// caller, since only it can tell that state from a genuinely new store.
	///
	fn secrets_script(&self, stack: &ResolvedStack) -> String {
		let template = r#"#!/bin/bash
# Render the bootstrap credential or the claimed store config at every start.
set -euo pipefail
umask 077
get() { aws ssm get-parameter --region '__REGION__' --name "$1" --with-decryption --query Parameter.Value --output text; }
if [ -f /etc/stalwart/config.json ] || [ "${1:-}" = "render-store" ]; then
	cp /etc/stalwart/config.json.template /etc/stalwart/config.json.next
	mv -f /etc/stalwart/config.json.next /etc/stalwart/config.json
	# the store is claimed, so a real administrator account exists and the
	# recovery credential is a second way in that nothing needs
	: > /etc/stalwart/stalwart.env.next
else
	admin_password="$(get '__ADMIN_SECRET__')"
	printf 'STALWART_RECOVERY_ADMIN=__ADMIN_USER__:%s\n' "$admin_password" > /etc/stalwart/stalwart.env.next
fi
mv -f /etc/stalwart/stalwart.env.next /etc/stalwart/stalwart.env
"#;
		let admin_secret = self.admin_secret_name(stack);
		[
			("__REGION__", stack.region().as_str()),
			("__ADMIN_SECRET__", admin_secret.as_str()),
			("__ADMIN_USER__", Self::ADMIN_USER),
		]
		.iter()
		.fold(template.to_string(), |script, (token, value)| {
			script.replace(token, value)
		})
		.trim_end()
		.to_string()
	}

	/// Returns the nightly SQLite backup script at
	/// `/usr/local/bin/stalwart-backup`.
	///
	/// SQLite's online `.backup` command produces a consistent snapshot while
	/// Stalwart continues serving. The script verifies that snapshot, uploads it,
	/// downloads it again, compares the bytes, verifies the downloaded database,
	/// and removes both local copies on every exit.
	fn backup_script(&self, stack: &ResolvedStack) -> String {
		let template = r#"#!/bin/bash
# Nightly consistent snapshot of the mail database into the archive bucket.
set -euo pipefail
umask 077
database='__DATABASE__'
snapshot="$(mktemp /var/lib/stalwart/backup.XXXXXX.db)"
readback="$(mktemp /var/lib/stalwart/backup-readback.XXXXXX.db)"
trap 'rm -f "$snapshot" "$readback"' EXIT
check_integrity() {
	result="$(sqlite3 "$1" 'PRAGMA integrity_check;')"
	[ "$result" = "ok" ] || {
		echo "SQLite integrity check failed for $1: $result" >&2
		exit 1
	}
}
sqlite3 "$database" ".backup '$snapshot'"
check_integrity "$snapshot"
object="s3://__BUCKET__/__PREFIX__/$(date -u +%Y/%m/%d/%H%M%SZ).db"
aws s3 cp "$snapshot" "$object" --only-show-errors --region '__REGION__'
aws s3 cp "$object" "$readback" --only-show-errors --region '__REGION__'
cmp --silent "$snapshot" "$readback" || {
	echo "S3 read verification differed from the uploaded SQLite snapshot" >&2
	exit 1
}
check_integrity "$readback"
echo "mail database backed up and read-verified at $object ($(stat -c %s "$snapshot") bytes)"
"#;
		let bucket = stack.resource_name(self.backup_bucket.clone());
		[
			("__DATABASE__", Self::DATABASE_PATH),
			("__REGION__", stack.region().as_str()),
			("__BUCKET__", bucket.as_str()),
			("__PREFIX__", Self::BACKUP_PREFIX),
		]
		.iter()
		.fold(template.to_string(), |script, (token, value)| {
			script.replace(token, value)
		})
		.trim_end()
		.to_string()
	}

	/// The backup unit and its timer.
	///
	/// `Persistent=true` so a box that was down at the scheduled hour dumps as
	/// soon as it is up rather than skipping the night, and a randomised delay
	/// so several of these never hit the database together. The unit logs where
	/// the mail server does, so a failed dump reaches the same CloudWatch group
	/// `watch` tails rather than only the box's journal.
	fn backup_units(&self) -> (String, String) {
		let service = r#"[Unit]
Description=Nightly SQLite snapshot of the mail database into S3
After=network-online.target
RequiresMountsFor=/var/lib/stalwart

[Service]
Type=oneshot
User=stalwart
Group=stalwart
ExecStart=/usr/local/bin/stalwart-backup
StandardOutput=append:/var/log/stalwart/stalwart.log
StandardError=append:/var/log/stalwart/stalwart.log"#
			.to_string();
		let timer = format!(
			r#"[Unit]
Description=Nightly mail database backup

[Timer]
OnCalendar={}
RandomizedDelaySec=900
Persistent=true

[Install]
WantedBy=timers.target"#,
			Self::BACKUP_SCHEDULE
		);
		(service, timer)
	}

	/// The systemd unit, modeled on the one Stalwart ships: unprivileged user
	/// with `CAP_NET_BIND_SERVICE` for the sub-1024 mail ports, SIGINT for a
	/// clean queue shutdown, secrets re-rendered on every start, and the log
	/// appended where the CloudWatch agent tails it.
	fn systemd_unit(&self) -> String {
		r#"[Unit]
Description=Stalwart Server
Conflicts=postfix.service sendmail.service exim4.service
After=network-online.target
RequiresMountsFor=/var/lib/stalwart

[Service]
Type=simple
LimitNOFILE=65536
KillMode=process
KillSignal=SIGINT
Restart=on-failure
RestartSec=5
User=stalwart
Group=stalwart
AmbientCapabilities=CAP_NET_BIND_SERVICE
ExecStartPre=/usr/local/bin/stalwart-secrets
EnvironmentFile=/etc/stalwart/stalwart.env
ExecStart=/usr/local/bin/stalwart --config=/etc/stalwart/config.json
StandardOutput=append:/var/log/stalwart/stalwart.log
StandardError=append:/var/log/stalwart/stalwart.log

[Install]
WantedBy=multi-user.target"#
			.to_string()
	}

	/// The CloudWatch agent config: tail the unit's log file into the block's
	/// group. `-m ec2` credentials come from the instance role; no config file
	/// of secrets exists. `timestamp_format` matches Stalwart's RFC3339 line
	/// prefix so events carry write time, and the same prefix keeps a
	/// multi-line backtrace one event.
	fn cloudwatch_config(&self, stack: &ResolvedStack) -> Value {
		json!({
			"agent": { "run_as_user": "root", "region": stack.region() },
			"logs": {
				"logs_collected": {
					"files": {
						"collect_list": [{
							"file_path": "/var/log/stalwart/stalwart.log",
							"log_group_name": self.log_group(stack),
							"log_stream_name": "stalwart",
							"retention_in_days": 30,
							"timestamp_format": "%Y-%m-%dT%H:%M:%S",
							"timezone": "UTC",
							"multi_line_start_pattern": "{timestamp_format}"
						}]
					}
				}
			}
		})
	}

	/// The cloud-init stanza that installs the backup script and its units, or
	/// nothing at all when no [`backup_bucket`](Self::backup_bucket) is
	/// declared.
	///
	fn backup_stanza(&self, stack: &ResolvedStack) -> String {
		if self.backup_bucket.is_empty() {
			return String::new();
		}
		let script = self.backup_script(stack);
		let (service, timer) = self.backup_units();
		format!(
			r#"# the nightly snapshot: the box holds the live database and network path, so
# the backup runs here rather than from whatever machine last deployed
cat > /usr/local/bin/stalwart-backup <<'BACKUP_EOF'
{script}
BACKUP_EOF
chmod 0755 /usr/local/bin/stalwart-backup

cat > /etc/systemd/system/stalwart-backup.service <<'BACKUP_UNIT_EOF'
{service}
BACKUP_UNIT_EOF

cat > /etc/systemd/system/stalwart-backup.timer <<'BACKUP_TIMER_EOF'
{timer}
BACKUP_TIMER_EOF
"#
		)
	}

	/// The line that starts the backup timer, or nothing when there is none.
	fn backup_enable(&self) -> &'static str {
		match self.backup_bucket.is_empty() {
			true => "",
			false => "systemctl enable --now stalwart-backup.timer",
		}
	}

	/// The cloud-init script, ie the machine's identity. Installs the pinned
	/// release (refusing a tarball that fails the pinned checksum), the store
	/// config template, the secrets renderer, the unit and the log agent; then
	/// renders secrets once and starts the service. The first boot comes up in
	/// Stalwart's bootstrap mode, serving management only, until
	/// `StalwartProvision` applies the declarative config.
	fn build_user_data(&self, stack: &ResolvedStack) -> Result<SmolStr> {
		let hostname = &self.hostname;
		let version = Self::STALWART_VERSION;
		let tarball = Self::STALWART_TARBALL;
		let sha256 = Self::STALWART_SHA256;
		let store_template =
			serde_json::to_string_pretty(&self.data_store_config())?;
		let secrets_script = self.secrets_script(stack);
		let unit = self.systemd_unit();
		let cloudwatch =
			serde_json::to_string_pretty(&self.cloudwatch_config(stack))?;
		let backup = self.backup_stanza(stack);
		let backup_enable = self.backup_enable();
		let data_mount = Self::DATA_MOUNT;

		let script = format!(
			r#"#!/bin/bash
set -euo pipefail

# the box's own name, ie what its SMTP banner and HELO identify as
hostnamectl set-hostname '{hostname}'

# Terraform may attach the data volume after cloud-init starts. Nitro exposes an
# EBS attachment by its volume id, not reliably by the requested /dev/sdf name,
# so wait for that exact by-id path and never risk formatting another disk.
dnf install -y e2fsprogs sqlite
data_volume_id='__DATA_VOLUME_ID__'
data_device="/dev/disk/by-id/nvme-Amazon_Elastic_Block_Store_${{data_volume_id//-/}}"
udevadm settle || true
for attempt in $(seq 1 300); do
	[ -b "$data_device" ] && break
	sleep 2
done
if [ ! -b "$data_device" ]; then
	echo "timed out waiting for Stalwart data volume $data_volume_id" >&2
	exit 1
fi

# A blank volume is first-deploy state; any known ext4 volume is persistent state.
# Refuse every other filesystem rather than turning a detection problem into loss.
# `blkid` exits 2 for "nothing found", ie a genuinely blank device; every other
# non-zero status is a failure to LOOK, which must never reach mkfs.
set +e
data_fs_type="$(blkid -s TYPE -o value "$data_device")"
data_fs_status=$?
set -e
if [ "$data_fs_status" -ne 0 ] && [ "$data_fs_status" -ne 2 ]; then
	echo "could not read the filesystem of Stalwart data volume $data_volume_id (blkid exit $data_fs_status)" >&2
	exit 1
fi
if [ -z "$data_fs_type" ]; then
	mkfs.ext4 -L stalwart-data "$data_device"
elif [ "$data_fs_type" != ext4 ]; then
	echo "Stalwart data volume $data_volume_id has unexpected filesystem $data_fs_type" >&2
	exit 1
fi
data_uuid="$(blkid -s UUID -o value "$data_device")"
if [ -z "$data_uuid" ]; then
	echo "Stalwart data volume $data_volume_id has no filesystem UUID" >&2
	exit 1
fi
mkdir -p '{data_mount}'
fstab_line="UUID=$data_uuid {data_mount} ext4 defaults,nofail,x-systemd.device-timeout=5min 0 2"
if grep -Eq '[[:space:]]{data_mount}[[:space:]]' /etc/fstab; then
	grep -qF "$fstab_line" /etc/fstab || {{
		echo "{data_mount} already has a different fstab entry" >&2
		exit 1
	}}
else
	printf '%s\n' "$fstab_line" >> /etc/fstab
fi
mountpoint -q '{data_mount}' || mount '{data_mount}'
findmnt -rn -S "UUID=$data_uuid" -T '{data_mount}' >/dev/null || {{
	echo "{data_mount} is not mounted from Stalwart data volume $data_volume_id" >&2
	exit 1
}}

# the service account and Stalwart's FHS layout
id stalwart >/dev/null 2>&1 || useradd --system --home {data_mount} --no-create-home --shell /usr/sbin/nologin stalwart
mkdir -p /etc/stalwart {data_mount} /var/log/stalwart
chown -R stalwart:stalwart /etc/stalwart {data_mount} /var/log/stalwart


# the pinned release: a tarball that does not hash to the pinned digest never
# reaches the disk, and the boot fails loudly instead. Retries cover a transient
# mid-transfer reset, which would otherwise dead-end the whole first boot: this
# script runs once per instance, so a flaky download costs a machine rebuild.
curl -sSLf --retry 5 --retry-all-errors --retry-delay 5 'https://github.com/stalwartlabs/stalwart/releases/download/v{version}/{tarball}' -o /tmp/stalwart.tar.gz
echo '{sha256}  /tmp/stalwart.tar.gz' | sha256sum -c -
tar -xzf /tmp/stalwart.tar.gz -C /usr/local/bin stalwart
chmod 0755 /usr/local/bin/stalwart
rm /tmp/stalwart.tar.gz

# the data store config as a TEMPLATE, never the file itself: config.json's
# absence is what puts the first boot in bootstrap mode, the server writes the
# real file when provision claims it, and every later start copies this static
# SQLite template back into place
cat > /etc/stalwart/config.json.template <<'CONFIG_EOF'
{store_template}
CONFIG_EOF
chown stalwart:stalwart /etc/stalwart/config.json.template

cat > /usr/local/bin/stalwart-secrets <<'SECRETS_EOF'
{secrets_script}
SECRETS_EOF
chmod 0755 /usr/local/bin/stalwart-secrets

cat > /etc/systemd/system/stalwart.service <<'UNIT_EOF'
{unit}
UNIT_EOF


# log forwarding into the block's own group, credentials from the instance role
dnf install -y amazon-cloudwatch-agent
cat > /opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json <<'CW_EOF'
{cloudwatch}
CW_EOF
/opt/aws/amazon-cloudwatch-agent/bin/amazon-cloudwatch-agent-ctl -a fetch-config -m ec2 -s -c file:/opt/aws/amazon-cloudwatch-agent/etc/amazon-cloudwatch-agent.json

{backup}
# secrets rendered before the first start, and by ExecStartPre on every later
# one, so rotation is a restart rather than a rebuild
sudo -u stalwart /usr/local/bin/stalwart-secrets
systemctl daemon-reload
systemctl enable --now stalwart
{backup_enable}
"#
		);

		// terraform reads user_data as a string, so every literal `${..}` must
		// escape to `$${..}` BEFORE the deliberate terraform refs land.
		let script = script.replace("${", "$${");
		let script =
			script.replace("__DATA_VOLUME_ID__", &self.data_volume_id(stack));
		SmolStr::from(script).xok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The mail box as the plan declares it: label `mail`, metadata on its data
	/// volume and blobs in `mail-blobs`.
	fn mail_box() -> StalwartBlock {
		StalwartBlock::new("mail", "mail.beetmash.com")
			.with_blob_bucket("mail-blobs")
			.with_ssh_public_key(
				"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITESTKEY pete",
			)
			.with_dns(DnsProvider::cloudflare("mail.beetmash.com", "zone123"))
	}

	/// The blocks the box is deployed beside, whose grants it lowers.
	fn siblings() -> (VpcBlock, S3BucketBlock) {
		(
			VpcBlock::new("net"),
			S3BucketBlock::new("mail-blobs")
				.with_deploy_versioned(false)
				.with_runtime_write(true),
		)
	}

	/// Spawn `block` beside its [`siblings`], related to its network and serving
	/// one SES-relayed domain.
	fn spawn_stack(block: StalwartBlock, parent: &mut ChildSpawner) {
		spawn_relayed(block, RelayMode::Ses(SesRelay::default()), parent);
	}

	/// [`spawn_stack`] with the served domain relayed through `relay`, ie what
	/// decides whether the box declares a SES sending identity at all.
	fn spawn_relayed(
		block: StalwartBlock,
		relay: RelayMode,
		parent: &mut ChildSpawner,
	) {
		let (network, blobs) = siblings();
		let vpc = parent.spawn(network).id();
		parent.spawn((block, VpcRef(vpc)));
		parent.spawn(blobs);
		relay.insert(
			&mut parent.spawn(
				MailDomainBlock::new(
					"stalwart.beetmash.com",
					"mail.beetmash.com",
				)
				.with_records(MailRecords::None),
			),
		);
	}

	/// The config the whole set emits with its domain relayed through `relay`.
	fn build_config_relayed(
		block: &StalwartBlock,
		relay: RelayMode,
	) -> terra::Config {
		let block = block.clone();
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				spawn_relayed(block, relay, parent);
			});
		scope.finish().unwrap().2
	}

	/// The Sydney stack every test renders against.
	fn sydney_stack() -> Stack {
		Stack::new("beet_infra").with_region(aws::region::AP_SOUTHEAST_2)
	}

	/// The config the whole set emits against `stack`.
	fn build_config_at(
		block: &StalwartBlock,
		stack: Stack,
	) -> (ResolvedStack, Deployment, terra::Config) {
		let block = block.clone();
		let (scope, _dir) = RenderScope::test_render_stack(stack, |parent| {
			spawn_stack(block, parent);
		});
		scope.finish().unwrap()
	}

	/// The config the whole set emits against a Sydney development stack.
	fn build_config(
		block: &StalwartBlock,
	) -> (ResolvedStack, Deployment, terra::Config) {
		build_config_at(block, sydney_stack())
	}

	/// The rendered user_data, ie the machine identity.
	fn user_data(block: &StalwartBlock) -> String {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		block.build_user_data(&stack).unwrap().to_string()
	}

	/// The sole resource of `resource_type` in the rendered config.
	fn resource(
		config: &terra::Config,
		resource_type: &str,
	) -> serde_json::Value {
		config.to_json().into_json()["resource"][resource_type]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()
			.clone()
	}

	/// The sole `aws_instance` in the rendered config.
	fn instance(config: &terra::Config) -> serde_json::Value {
		resource(config, "aws_instance")
	}

	/// A code-only deploy renders the identical config, so terraform plans no
	/// change and nothing rebuilds: the box's identity carries no deploy id,
	/// and every per-deploy value lives in SSM or in the database.
	#[beet_core::test]
	fn code_only_deploy_renders_one_box() {
		let (deployment, _dir) = Deployment::default_local();
		let second = deployment
			.clone()
			.with_deploy_id(uuid_ext::now_v7())
			.with_deploy_timestamp("2026-08-27T00:00:00Z".to_string());
		// [`RenderScope::test_render_stack`] mints its own deployment, so this
		// render seeds the schedule with the caller's directly
		let render = |deployment: &Deployment| {
			let mut world = InfraPlugin.into_world();
			world.insert_resource(deployment.clone());
			world.init_resource::<PackageConfig>();
			let root = world
				.spawn(sydney_stack())
				.with_children(|parent| {
					spawn_stack(mail_box(), parent);
				})
				.id();
			RenderScope::render(&mut world, root)
				.unwrap()
				.finish()
				.unwrap()
				.2
				.to_json_string()
				.unwrap()
		};
		render(&deployment).xpect_eq(render(&second));
		user_data(&mail_box())
			.as_str()
			.xnot()
			.xpect_contains(&deployment.deploy_id().to_string());
	}

	/// The rebuild rule is a terraform setting, not a convention: an edited
	/// user_data REPLACES the instance rather than mutating a live MTA.
	#[beet_core::test]
	fn machine_config_change_replaces_the_instance() {
		let (_stack, _deployment, config) = build_config(&mail_box());
		instance(&config)["user_data_replace_on_change"]
			.as_bool()
			.unwrap()
			.xpect_true();
		// ..and a changed hostname IS a changed machine
		user_data(&mail_box()).xpect_not_eq(user_data(
			&StalwartBlock::new("mail", "mx.beetmash.com")
				.with_blob_bucket("mail-blobs")
				.with_ssh_public_key("ssh-ed25519 KEY pete"),
		));
	}

	/// The install is pinned twice: the exact release in the url, and the
	/// digest the downloaded bytes must hash to. `sha256sum -c` failing aborts
	/// the boot under `set -e`, so a tampered or moved tarball is a dead box
	/// rather than a mystery MTA.
	#[beet_core::test]
	fn checksum_gates_the_install() {
		user_data(&mail_box())
			.as_str()
			.xpect_contains(&format!(
				"releases/download/v{}/{}",
				StalwartBlock::STALWART_VERSION,
				StalwartBlock::STALWART_TARBALL
			))
			.xpect_contains(&format!(
				"echo '{}  /tmp/stalwart.tar.gz' | sha256sum -c -",
				StalwartBlock::STALWART_SHA256
			));
	}

	/// No secret exists in machine config or transits it: the only terraform
	/// interpolation in rendered user_data is the data volume ID, while the
	/// script holds the bootstrap admin's SSM parameter name, never its value.
	/// The SES SMTP secret appears only as the `aws_ssm_parameter` resource's
	/// interpolation, which is state-bound, never user_data-bound.
	#[beet_core::test]
	fn no_secret_material_in_machine_config() {
		let block = mail_box();
		let script = user_data(&block);
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let volume = block
			.data_volume_id(&stack)
			.trim_end_matches('}')
			.to_string();
		script
			.match_indices("${")
			.filter(|(index, _)| !script[..*index].ends_with('$'))
			.map(|(index, _)| {
				script[index..].split('}').next().unwrap().to_string()
			})
			.collect::<Vec<_>>()
			.xpect_eq(vec![volume]);
		script
			.as_str()
			.xnot()
			.xpect_contains("ses_smtp_password_v4")
			.xnot()
			.xpect_contains("aws_iam_access_key")
			.xnot()
			.xpect_contains("AWS_ACCESS_KEY")
			.xnot()
			.xpect_contains("AWS_SECRET");
	}

	/// The boot script reads only the bootstrap admin password `EnsureSecret`
	/// creates. SQLite has no database credential to fetch or expose.
	#[beet_core::test]
	fn boot_reads_only_the_bootstrap_admin_parameter() {
		let block = mail_box();
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		user_data(&block)
			.as_str()
			.xpect_contains(&format!(
				"get '{}'",
				block.admin_secret_name(&stack)
			))
			.xnot()
			.xpect_contains("db-password")
			.xpect_contains("--with-decryption")
			.xpect_contains("STALWART_RECOVERY_ADMIN=admin:");
		block
			.admin_secret_name(&stack)
			.as_str()
			.xpect_eq("/beet-infra/dev/mail-admin-password");
	}

	/// The SES SMTP credential is derived by terraform and parked in
	/// SecureString parameters under the same stack prefix, named by the same
	/// methods provision will read them through; the password value is the
	/// access key's `ses_smtp_password_v4` and exists nowhere else.
	#[beet_core::test]
	fn ses_smtp_credential_is_terraform_derived() {
		let block = mail_box();
		let (stack, _deployment, config) = build_config(&block);
		let params =
			config.to_json().into_json()["resource"]["aws_ssm_parameter"]
				.as_object()
				.unwrap()
				.values()
				.cloned()
				.collect::<Vec<_>>();
		params.len().xpect_eq(2);
		let value_of = |name: &str| {
			params
				.iter()
				.find(|param| param["name"] == name)
				.unwrap()
				.clone()
		};
		let user = value_of(&block.ses_smtp_user_secret_name(&stack));
		let password = value_of(&block.ses_smtp_password_secret_name(&stack));
		user["type"].as_str().unwrap().xpect_eq("SecureString");
		password["type"].as_str().unwrap().xpect_eq("SecureString");
		password["value"]
			.as_str()
			.unwrap()
			.xpect_contains(".ses_smtp_password_v4}");
		// the sending user may send raw mail, and do nothing else
		let policy =
			config.to_json().into_json()["resource"]["aws_iam_user_policy"]
				.as_object()
				.unwrap()
				.values()
				.next()
				.unwrap()["policy"]
				.as_str()
				.unwrap()
				.to_string();
		policy
			.as_str()
			.xpect_contains("ses:SendRawEmail")
			.xnot()
			.xpect_contains("ses:*");
	}

	/// The SES sending identity exists exactly while some domain relays through
	/// SES: an IAM user holding a static credential is the one thing in this
	/// stack that exists only because a provider's protocol demands it, so a
	/// stack that relays through comail or delivers directly holds no static
	/// credential at all.
	#[beet_core::test]
	fn no_ses_domain_means_no_ses_sender() {
		for relay in
			[RelayMode::None, RelayMode::Comail(ComailRelay::default())]
		{
			build_config_relayed(&mail_box(), relay)
				.to_json_string()
				.unwrap()
				.as_str()
				.xnot()
				.xpect_contains("aws_iam_user")
				.xnot()
				.xpect_contains("aws_iam_access_key")
				.xnot()
				.xpect_contains("ses-smtp-user");
		}
		// ..and the box is still a box: the instance and its role are not the
		// relay's to take away
		build_config_relayed(&mail_box(), RelayMode::None)
			.to_json_string()
			.unwrap()
			.as_str()
			.xpect_contains("aws_instance")
			.xpect_contains("aws_iam_role");
	}

	/// The firewall is the port list and nothing else: one tcp ingress per
	/// mail service, one allow-all egress, and no management port. Bootstrap
	/// mode's 8080 is reached through the SSH port, never the internet.
	#[beet_core::test]
	fn firewall_is_exactly_the_port_list() {
		let (_stack, _deployment, config) = build_config(&mail_box());
		let rules =
			config.to_json().into_json()["resource"]["aws_security_group_rule"]
				.as_object()
				.unwrap()
				.values()
				.cloned()
				.collect::<Vec<_>>();
		let mut ingress = rules
			.iter()
			.filter(|rule| rule["type"] == "ingress")
			// the box's public rules, not the database's group-sourced one
			.filter(|rule| rule["cidr_blocks"][0] == "0.0.0.0/0")
			.map(|rule| rule["from_port"].as_i64().unwrap())
			.collect::<Vec<_>>();
		ingress.sort();
		ingress.xpect_eq(vec![22, 25, 443, 465, 587, 993]);
		let egress = rules
			.iter()
			.filter(|rule| rule["type"] == "egress")
			.collect::<Vec<_>>();
		egress.len().xpect_eq(1);
		egress[0]["protocol"].as_str().unwrap().xpect_eq("-1");
		config
			.to_json_string()
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("8080");
	}

	/// IMDSv2 required, encrypted gp3 root at the declared size, Graviton
	/// sizing, and the arm64 AMI pointer: the hardware half of the security
	/// practices list, pinned.
	#[beet_core::test]
	fn instance_hardening_is_not_default() {
		let (_stack, _deployment, config) = build_config(&mail_box());
		let instance = instance(&config);
		instance["metadata_options"][0]["http_tokens"]
			.as_str()
			.unwrap()
			.xpect_eq("required");
		let root = &instance["root_block_device"][0];
		root["encrypted"].as_bool().unwrap().xpect_true();
		root["volume_type"].as_str().unwrap().xpect_eq("gp3");
		root["volume_size"].as_i64().unwrap().xpect_eq(30);
		instance["instance_type"]
			.as_str()
			.unwrap()
			.xpect_eq("t4g.small");
		// the AMI rides the public AL2023 arm64 pointer, resolved per apply
		instance["ami"]
			.as_str()
			.unwrap()
			.xpect_contains("${data.aws_ssm_parameter.");
		config.to_json().into_json()["data"]["aws_ssm_parameter"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()["name"]
			.as_str()
			.unwrap()
			.xpect_eq(StalwartBlock::AMI_PARAMETER)
			.xpect_contains("al2023")
			.xpect_contains("arm64");
	}

	/// Production protects the persistent data volume and takes a final snapshot,
	/// while the attachment remains replaceable with the cattle box.
	#[beet_core::test]
	fn production_data_volume_and_replaceable_attachment_are_protected() {
		let (stack, _deployment, config) = build_config_at(
			&mail_box(),
			sydney_stack().with_stage(BootstrapConfig::PROD_STAGE),
		);
		let volume = resource(&config, "aws_ebs_volume");
		// the subnet's zone by reference, never a second literal: a volume in a
		// zone the box is not in is an attachment that fails at apply
		volume["availability_zone"].as_str().unwrap().xpect_eq(
			VpcBlock::new("net").subnet_availability_zone(
				&stack,
				SubnetTier::Public,
				"a",
			),
		);
		volume["encrypted"].as_bool().unwrap().xpect_true();
		volume["final_snapshot"].as_bool().unwrap().xpect_true();
		volume["type"].as_str().unwrap().xpect_eq("gp3");
		volume["size"].as_i64().unwrap().xpect_eq(20);
		volume["lifecycle"]["prevent_destroy"]
			.as_bool()
			.unwrap()
			.xpect_true();
		volume["tags"]["Name"]
			.as_str()
			.unwrap()
			.xpect_eq("mail--data");
		volume["tags"]["Project"]
			.as_str()
			.unwrap()
			.xpect_eq("beet_infra");

		let attachment = resource(&config, "aws_volume_attachment");
		attachment["device_name"]
			.as_str()
			.unwrap()
			.xpect_eq(StalwartBlock::DATA_DEVICE_NAME);
		attachment["stop_instance_before_detaching"]
			.as_bool()
			.unwrap()
			.xpect_true();
		attachment["volume_id"]
			.as_str()
			.unwrap()
			.xpect_eq(mail_box().data_volume_id(&stack));
		attachment["lifecycle"]["replace_triggered_by"][0]
			.as_str()
			.unwrap()
			.xpect_contains("aws_instance")
			.xpect_contains(".id");
	}

	/// Non-production stages are disposable by default so a restore drill can
	/// destroy the same declaration after proving the backup. An explicit override
	/// can make even production disposable for a purpose-built temporary stack.
	#[beet_core::test]
	fn data_volume_protection_follows_the_stage_unless_overridden() {
		let (_stack, _deployment, config) = build_config(&mail_box());
		let volume = resource(&config, "aws_ebs_volume");
		volume["final_snapshot"].as_bool().unwrap().xpect_false();
		volume["lifecycle"]["prevent_destroy"]
			.is_null()
			.xpect_true();

		let (_stack, _deployment, config) = build_config_at(
			&mail_box().with_data_volume_protected(false),
			sydney_stack().with_stage(BootstrapConfig::PROD_STAGE),
		);
		let volume = resource(&config, "aws_ebs_volume");
		volume["final_snapshot"].as_bool().unwrap().xpect_false();
		volume["lifecycle"]["prevent_destroy"]
			.is_null()
			.xpect_true();
	}

	/// The mail store is the one thing on this box that cannot be rebuilt, so
	/// cloud-init waits for the volume BY ID, formats only a device it has
	/// positively established is blank, and mounts by filesystem UUID before
	/// anything else touches the directory.
	#[beet_core::test]
	fn cloud_init_formats_once_and_mounts_by_uuid_before_stalwart() {
		let block = mail_box().with_backup_bucket("archive");
		let script = user_data(&block);
		script
			.as_str()
			.xpect_contains(
				"nvme-Amazon_Elastic_Block_Store_$${data_volume_id//-/}",
			)
			.xpect_contains("if [ -z \"$data_fs_type\" ]; then\n\tmkfs.ext4")
			.xpect_contains("findmnt -rn -S \"UUID=$data_uuid\"")
			.xpect_contains("RequiresMountsFor=/var/lib/stalwart");
		let mount = script.find("mountpoint -q").unwrap();
		let user = script.find("useradd --system").unwrap();
		(mount < user).xpect_true();
		block
			.backup_units()
			.0
			.as_str()
			.xpect_contains("RequiresMountsFor=/var/lib/stalwart");
	}

	/// `blkid` exits 2 when it finds no filesystem and non-zero for every other
	/// reason, so an empty answer alone cannot be the format condition: a
	/// transient failure to READ a populated volume would otherwise mkfs the
	/// mail store. `set -e` would abort the script before the status could be
	/// examined, hence the guarded pair around exactly that one command.
	#[beet_core::test]
	fn a_failure_to_detect_a_filesystem_never_reaches_mkfs() {
		let script = user_data(&mail_box());
		let guarded = script
			.split_once("set +e")
			.unwrap()
			.1
			.split_once("set -e")
			.unwrap()
			.0;
		guarded
			.xpect_contains("blkid -s TYPE")
			.xpect_contains("data_fs_status=$?");
		script
			.as_str()
			.xpect_contains(
				"[ \"$data_fs_status\" -ne 0 ] && [ \"$data_fs_status\" -ne 2 ]",
			)
			.xnot()
			.xpect_contains("blkid -s TYPE -o value \"$data_device\" || true");
	}

	/// The mount is `nofail`: a volume that does not appear must leave a box
	/// that boots and can be reached over ssh, not one in an emergency shell
	/// with no way in. `RequiresMountsFor` is what keeps Stalwart from starting
	/// against a missing store, so the two are one decision.
	#[beet_core::test]
	fn a_missing_volume_leaves_a_reachable_box() {
		user_data(&mail_box())
			.as_str()
			.xpect_contains(
				"UUID=$data_uuid /var/lib/stalwart ext4 \
				 defaults,nofail,x-systemd.device-timeout=5min 0 2",
			)
			.xpect_contains("RequiresMountsFor=/var/lib/stalwart");
	}

	#[beet_core::test]
	fn public_subnet_eip_and_the_shared_group_label() {
		let (stack, _deployment, config) = build_config(&mail_box());
		instance(&config)["subnet_id"].as_str().unwrap().xpect_eq(
			VpcBlock::new("net").subnet_id(&stack, SubnetTier::Public, "a"),
		);
		config.to_json().into_json()["resource"]["aws_security_group"]
			.as_object()
			.unwrap()
			.contains_key(mail_box().security_group_ident(&stack).label())
			.xpect_true();
		// the eip waits on the gateway the vpc declares
		config.to_json().into_json()["resource"]["aws_eip"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()["depends_on"][0]
			.as_str()
			.unwrap()
			.xpect_contains("aws_internet_gateway");
		config.to_json().into_json()["resource"]["aws_eip_association"]
			.as_object()
			.unwrap()
			.len()
			.xpect_eq(1);
	}

	/// The role is lowered from what the siblings declared: its bootstrap-admin
	/// parameter, write access only on the blob bucket that declared it, its own
	/// log group, and no static credential anywhere.
	#[beet_core::test]
	fn role_lowers_the_declared_grants() {
		let (stack, _deployment, config) = build_config(&mail_box());
		let policy =
			config.to_json().into_json()["resource"]["aws_iam_role_policy"]
				.as_object()
				.unwrap()
				.values()
				.next()
				.unwrap()["policy"]
				.as_str()
				.unwrap()
				.to_string();
		policy
			.as_str()
			// the one parameter the box reads during bootstrap
			.xpect_contains(
				"arn:aws:ssm:ap-southeast-2:*:parameter/beet-infra/dev/mail-admin-password",
			)
			.xnot()
			.xpect_contains("parameter/beet-infra/dev/*")
			.xnot()
			.xpect_contains("db-password")
			// blob bucket readable and writable, by declaration
			.xpect_contains(&format!(
				"arn:aws:s3:::{}/*",
				stack.resource_name("mail-blobs")
			))
			.xpect_contains("s3:PutObject")
			.xpect_contains(&mail_box().log_group(&stack))
			// and never the account
			.xnot()
			.xpect_contains("\"Resource\":\"arn:aws:s3:::*\"");
		// grants scale with declarations: an undeclared write is not granted
		serde_json::from_str::<serde_json::Value>(&policy).unwrap()["Statement"]
			.as_array()
			.unwrap()
			.iter()
			.filter(|statement| statement["Sid"] == "WriteStores")
			.count()
			.xpect_eq(1);
	}

	/// The on-disk config template is Stalwart `0.16`'s `DataStore` object
	/// alone: an internally tagged SQLite store on the persistent volume, with no
	/// credential or other store in the file.
	#[beet_core::test]
	fn data_store_config_is_static_sqlite() {
		let config = mail_box().data_store_config();
		config["@type"].as_str().unwrap().xpect_eq("Sqlite");
		config["path"]
			.as_str()
			.unwrap()
			.xpect_eq(StalwartBlock::DATABASE_PATH);
		config.get("authSecret").is_none().xpect_true();
		for absent in ["DataStore", "SearchStore", "InMemoryStore", "BlobStore"]
		{
			config[absent].is_null().xpect_true();
		}
	}

	/// The recovery credential exists ONLY while the data store is unclaimed:
	/// once a `config.json` is on disk there is a real administrator account,
	/// and a second password that provisions the whole server over a plaintext
	/// port is a backdoor with no remaining purpose.
	#[beet_core::test]
	fn the_recovery_credential_retires_with_the_claim() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let script = mail_box().secrets_script(&stack);
		// the claimed branch renders the store and an EMPTY env file
		let (claimed, unclaimed) = script
			.split_once("else")
			.map(|(claimed, unclaimed)| {
				(claimed.to_string(), unclaimed.to_string())
			})
			.unwrap();
		claimed.as_str().xpect_contains("config.json.next");
		claimed
			.as_str()
			.xnot()
			.xpect_contains("STALWART_RECOVERY_ADMIN");
		unclaimed.as_str().xpect_contains("STALWART_RECOVERY_ADMIN");
	}

	/// A rebuilt box has a fresh root disk and an existing SQLite data store on
	/// its persistent volume, so bootstrap mode is a lie the missing file told.
	/// `render-store` is how provision tells the box otherwise.
	#[beet_core::test]
	fn the_store_render_can_be_forced() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		mail_box()
			.secrets_script(&stack)
			.as_str()
			.xpect_contains("render-store");
	}

	/// The nightly dump is opt-in and complete: no bucket, no timer, and no
	/// half-installed script that fails every night at half past two.
	#[beet_core::test]
	fn the_backup_timer_is_declared_or_absent() {
		user_data(&mail_box())
			.as_str()
			.xnot()
			.xpect_contains("stalwart-backup");
		user_data(&mail_box().with_backup_bucket("archive"))
			.as_str()
			.xpect_contains("/usr/local/bin/stalwart-backup")
			.xpect_contains("stalwart-backup.timer")
			.xpect_contains("sqlite3")
			.xpect_contains(StalwartBlock::BACKUP_SCHEDULE);
	}

	/// The online SQLite backup is checked before upload and again after S3
	/// readback, with byte equality between those two verified files.
	#[beet_core::test]
	fn backup_is_consistent_and_read_verified() {
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		mail_box()
			.with_backup_bucket("archive")
			.backup_script(&stack)
			.as_str()
			.xpect_contains(&format!(
				"database='{}'",
				StalwartBlock::DATABASE_PATH
			))
			.xpect_contains(".backup '$snapshot'")
			.xpect_contains("PRAGMA integrity_check;")
			.xpect_contains("cmp --silent")
			.xpect_contains(&format!(
				"s3://{}/{}/",
				stack.resource_name("archive"),
				StalwartBlock::BACKUP_PREFIX
			))
			.xpect_contains(".db");
	}

	/// The blob store the `Bootstrap` claim declares: the stack's bucket with
	/// no static key anywhere (every credential slot is `None`, so the S3
	/// client falls through its chain to the instance profile).
	#[beet_core::test]
	fn blob_store_config_holds_s3_and_no_secrets() {
		let block = mail_box();
		let (stack, _deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		let blob = block.blob_store_config(&stack);
		blob["@type"].as_str().unwrap().xpect_eq("S3");
		blob["bucket"]
			.as_str()
			.unwrap()
			.xpect_eq(stack.resource_name("mail-blobs").as_str());
		blob["region"]["@type"]
			.as_str()
			.unwrap()
			.xpect_eq("ApSoutheast2");
		for slot in ["accessKey", "secretKey", "securityToken", "sessionToken"]
		{
			blob[slot]["@type"].as_str().unwrap().xpect_eq("None");
		}
	}

	/// The `A` record is the box's hostname at the EIP, DNS-only: an MTA
	/// behind a proxy is unreachable on 25 and cannot pass TLS-ALPN-01, so a
	/// proxied declaration fails at config time instead.
	#[beet_core::test]
	fn dns_is_an_unproxied_a_record() {
		let (_stack, _deployment, config) = build_config(&mail_box());
		let record =
			config.to_json().into_json()["resource"]["cloudflare_dns_record"]
				.as_object()
				.unwrap()
				.values()
				.next()
				.unwrap()
				.clone();
		record["type"].as_str().unwrap().xpect_eq("A");
		record["name"]
			.as_str()
			.unwrap()
			.xpect_eq("mail.beetmash.com");
		record["proxied"].as_bool().unwrap().xpect_false();
		record["content"]
			.as_str()
			.unwrap()
			.xpect_contains("aws_eip.")
			.xpect_contains(".public_ip}");
		mail_box()
			.with_dns(
				DnsProvider::cloudflare("mail.beetmash.com", "zone123")
					.with_proxied(true),
			)
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("must not be Cloudflare-proxied");
	}

	/// A declaration that cannot serve mail fails before any resource exists,
	/// and a box with no network to sit in fails when its relation resolves.
	#[beet_core::test]
	fn invalid_declarations_fail_at_config_time() {
		StalwartBlock::new("mail", "mail.beetmash.com")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("names no blob bucket");
		mail_box()
			.with_ssh_public_key("")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("no ssh public key");
		StalwartBlock::new("mail", "Mail.Beetmash.Com")
			.with_blob_bucket("mail-blobs")
			.with_ssh_public_key("ssh-ed25519 KEY")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("hostname");
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				parent.spawn(mail_box());
			});
		scope
			.finish()
			.unwrap_err()
			.to_string()
			.xpect_contains("declares no `VpcRef`");
	}

	/// The full stack through the real provider schemas: vpc, blob bucket and
	/// box in one config, related exactly as the markup relates them.
	/// Rendered-JSON assertions prove what the blocks meant; only tofu proves
	/// the schema accepts it and every cross-block interpolation resolves.
	///
	/// Drives the native tofu cli, so it cannot compile for wasm.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test(timeout_ms = 240000)]
	#[ignore = "very slow"]
	async fn validate() {
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				spawn_stack(mail_box(), parent);
			});
		scope.project().unwrap().validate().await.unwrap();
	}
}
