use crate::bindings::*;
use crate::mail::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;
use serde_json::json;

/// Which of a mail domain's records a [`MailDomainBlock`] publishes.
///
/// A domain's records fall into two groups with very different blast radius,
/// and the distinction is what makes a cutover a short window rather than a
/// second build.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect,
)]
#[reflect(Default)]
pub enum MailRecords {
	/// Everything: this stack serves the domain's mail.
	#[default]
	All,
	/// Only the records that prove the sending identity: the relay's selectors
	/// (and, for SES, its custom MAIL FROM pair) beside the sovereign one. All
	/// are namespaced, by selector or under a `bounce.` subdomain, so they
	/// coexist with a third party still serving the domain's mail. This is a
	/// cutover prepared ahead of its window: the identity verifies, and not one
	/// message changes hands.
	IdentityOnly,
	/// None. The identity exists and its records are published elsewhere.
	None,
}

impl MailRecords {
	/// Whether the records that hand this domain's mail to this stack are
	/// published: its `MX`, its SPF, its DMARC policy and the client discovery
	/// records.
	pub fn serves_mail(&self) -> bool { matches!(self, Self::All) }

	/// Whether the identity-scoped records are published.
	pub fn proves_identity(&self) -> bool {
		matches!(self, Self::All | Self::IdentityOnly)
	}
}

/// One mail domain, end to end: every DNS record that makes it deliverable and
/// discoverable, and the sending identity behind whichever relay it composes.
///
/// The domain is the only thing declared. Every record name, every resource
/// name and the MTA-STS host all derive from it, so serving a second domain is
/// a second instance rather than an edit, and the eventual cutover from a
/// staging domain to the apex is one more instance beside the others.
///
/// What the domain does NOT declare is which relay carries its outbound mail:
/// that is a [`SesRelay`] or [`ComailRelay`] composed beside it (or on an
/// ancestor), and absent both the box delivers straight to each recipient's MX.
/// See [`RelayMode`].
///
/// The domain a mailbox lives on is not the host that serves it: `mail_host` is
/// infrastructure and stays put while the domains it serves come and go, which
/// is what keeps its rDNS, its certificate and its SMTP banner stable across a
/// cutover.
///
/// Two records a reader may expect here are deliberately elsewhere. The
/// `mail_host` address record belongs to whichever block owns the box, since
/// only that block knows the address. The zone's `CAA` rows are a zone-wide
/// singleton owned by the site stack, and the `letsencrypt.org` pair the mail
/// box's ACME needs is already among them: a second block emitting its own
/// would fight the first for the same record rather than add to it.
#[derive(
	Debug, Clone, Get, SetWith, Serialize, Deserialize, Component, Reflect,
)]
#[reflect(Component, Default)]
#[component(immutable, on_insert = ErasedBlock::on_insert::<Self>,
	on_remove = ErasedBlock::on_remove
)]
pub struct MailDomainBlock {
	/// The mail domain served, eg `stalwart.beetmash.com`.
	domain: SmolStr,
	/// The host every `MX`, `SRV` and autoconfig record resolves to, ie the
	/// box. Infrastructure rather than a mail domain, so several domains name
	/// the same one.
	mail_host: SmolStr,
	/// See [`MailRecords`].
	records: MailRecords,
	/// The zone the records are published into. Without one the block declares
	/// the sending identity and nothing else, ie a domain whose records are
	/// held by somebody else.
	#[get(skip)]
	#[set_with(unwrap_option)]
	dns: Option<DnsProvider>,
	/// The domain the DMARC aggregate and TLS-RPT report addresses live on,
	/// defaulting to this domain's own. Reports have to reach a mailbox that
	/// exists, which while a domain is being commissioned is not that domain's
	/// own.
	#[get(skip)]
	#[set_with(unwrap_option, into)]
	report_domain: Option<SmolStr>,
	/// See [`MtaStsPolicy`].
	mta_sts: MtaStsPolicy,
	/// The mailboxes this domain stores mail for, read by `StalwartProvision`.
	#[set_with(skip)]
	mailboxes: Vec<Mailbox>,
	/// The localparts that redirect into those mailboxes, read by
	/// `StalwartProvision`.
	#[set_with(skip)]
	aliases: Vec<Alias>,
	/// The members whose identities this domain carries, read by
	/// `StalwartProvision`.
	#[set_with(skip)]
	members: Vec<Member>,
	/// The mailbox every otherwise unmatched address on this domain delivers
	/// to. A publication domain wants one (replies to a newsletter are welcome
	/// and unpredictable); a person's domain does not (a catch-all is a spam
	/// trap that accepts).
	#[set_with(unwrap_option, into)]
	catch_all: Option<SmolStr>,
	/// The SNS topic, by bare name in this account and region, this domain's
	/// reputation alarms notify. Without one nothing consumes the numbers,
	/// which is a sending domain whose first complaint is discovered by a
	/// suspension.
	///
	/// Provider-neutral: every relay has a bounce rate and a complaint rate,
	/// and both arms fire their alarms here. The SES event stream is a
	/// different thing and names its own topic
	/// ([`SesRelay::events_topic`]), because only SES emits it.
	///
	/// A name rather than a block reference because one topic serves every
	/// domain in the account: an alarm carries the domain it fired for, so a
	/// topic per domain would only move routing into IAM.
	#[set_with(unwrap_option, into)]
	alarms_topic: Option<SmolStr>,
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
	/// So a stage that is not this one emits its sending identity and its
	/// infrastructure and touches no record at all. Empty means no guard,
	/// which is right for a stack whose domain nothing else serves.
	#[set_with(unwrap_option, into)]
	dns_stage: Option<SmolStr>,
}

impl Default for MailDomainBlock {
	fn default() -> Self { Self::new("", "") }
}

impl MailDomainBlock {
	/// The single-MX preference. Nothing weighs against anything, so the value
	/// is convention rather than a choice.
	pub const MX_PRIORITY: u16 = 10;

	/// The selector the SOVEREIGN key signs under, ie the one this stack holds
	/// rather than the three Amazon rotates. Named for the server that signs
	/// with it, so a reader of the zone can tell the two sources apart at a
	/// glance.
	pub const DKIM_SELECTOR: &'static str = "stalwart";

	/// The client-facing services advertised by `SRV`, as
	/// `(service name, port)`. JMAP is first because it is the one an agent
	/// uses; IMAPS and submissions are what a mail client still asks for.
	pub const SERVICES: &'static [(&'static str, u16)] = &[
		("_jmap._tcp", 443),
		("_imaps._tcp", 993),
		("_submissions._tcp", 465),
	];

	/// The hostnames a mail client probes to configure itself, all pointing at
	/// the box, which serves both protocols on 443.
	pub const AUTOCONFIG_LABELS: &'static [&'static str] =
		&["autoconfig", "autodiscover"];

	/// A domain served by `mail_host`, publishing every record and addressing
	/// its reports to itself. A domain being commissioned overrides
	/// [`report_domain`](Self::with_report_domain).
	pub fn new(
		domain: impl Into<SmolStr>,
		mail_host: impl Into<SmolStr>,
	) -> Self {
		let domain = domain.into();
		Self {
			mail_host: mail_host.into(),
			records: MailRecords::All,
			dns: None,
			report_domain: None,
			mta_sts: default(),
			mailboxes: Vec::new(),
			aliases: Vec::new(),
			members: Vec::new(),
			catch_all: None,
			alarms_topic: None,
			dns_stage: None,
			domain,
		}
	}

	/// Where DMARC aggregate and TLS-RPT reports are addressed: the declared
	/// [`report_domain`](Self::with_report_domain), else this domain.
	///
	/// Resolved on read rather than copied at construction, since a markup
	/// declaration patches the domain over a default that has none, and a
	/// report address at the empty domain is a report nobody receives.
	pub fn report_domain(&self) -> &SmolStr {
		self.report_domain.as_ref().unwrap_or(&self.domain)
	}

	/// The zone this domain's records are published into: the declared
	/// [`dns`](Self::with_dns) provider, else a Cloudflare zone read from
	/// `CLOUDFLARE_ZONE_ID`.
	///
	/// A declaration says which domain it serves and nothing about where the
	/// zone lives, exactly as it says nothing about which AWS account it
	/// deploys into: both are properties of the launch. `None` is the honest
	/// answer for a domain whose records somebody else publishes, which is
	/// [`MailRecords::None`]'s whole meaning.
	pub fn resolved_dns(&self) -> Option<DnsProvider> {
		if let Some(dns) = &self.dns {
			return Some(dns.clone());
		}
		#[cfg(feature = "cloudflare_dns")]
		return DnsProvider::cloudflare_env(self.domain.clone());
		#[cfg(not(feature = "cloudflare_dns"))]
		None
	}

	/// Whether this stage may publish these names, ie whether it is the
	/// [`dns_stage`](Self::dns_stage) (or none was declared).
	pub fn owns_names(&self, stack: &ResolvedStack) -> bool {
		self.dns_stage
			.as_ref()
			.is_none_or(|owner| owner == stack.stage())
	}

	/// Add a member, and the mailbox they own on this domain.
	pub fn with_member(self, member: Member) -> Self {
		self.push_member(member, false)
	}

	/// Add a member whose mailbox also administers the server, ie whoever runs
	/// it. Distinct from an ordinary member because full management access is
	/// the one grant that should be tedious to hand out: it is rarely more than
	/// one account, and it must be a decision rather than a default.
	pub fn with_admin_member(self, member: Member) -> Self {
		self.push_member(member, true)
	}

	fn push_member(mut self, member: Member, admin: bool) -> Self {
		self.mailboxes
			.push(Mailbox::for_member(&member).with_admin(admin));
		self.members.push(member);
		self
	}

	/// Add a mailbox belonging to no member, ie a role address.
	pub fn with_mailbox(mut self, mailbox: Mailbox) -> Self {
		self.mailboxes.push(mailbox);
		self
	}

	/// Add one alias.
	pub fn with_alias(mut self, alias: Alias) -> Self {
		self.aliases.push(alias);
		self
	}

	/// Add the [`role alias`](Alias::roles) set, operator mail to `operator` and
	/// authentication reports to `reports`.
	pub fn with_role_aliases(mut self, operator: &str, reports: &str) -> Self {
		self.aliases.extend(Alias::roles(operator, reports));
		self
	}

	/// The domain with its dots as hyphens, ie `stalwart-beetmash-com`. Used
	/// wherever a name must be unique per domain but cannot contain a dot: the
	/// terraform labels, the SES configuration set name, and every parameter
	/// holding a mailbox credential.
	pub fn slug(&self) -> String { Self::slug_of(&self.domain) }

	/// [`slug`](Self::slug) for a domain no block declares, ie one named by a
	/// step rather than read off the stack. The restore drill is the only
	/// caller: the account it signs in as belongs to a domain the SOURCE stage
	/// serves, and composing that parameter name a second way is how the
	/// composition drifts.
	pub fn slug_of(domain: &str) -> String { domain.replace('.', "-") }

	/// The SES configuration set this domain sends through, ie the SES arm's
	/// per-domain reporting scope. Composed here rather than at each call site
	/// because a probe and an alarm must name the same set.
	pub fn configuration_set_name(&self) -> String {
		SesRelay::configuration_set_name(&self.slug())
	}

	/// The DMARC policy: reject outright, and report. `p=reject` from the first
	/// deploy is deliberate, since a policy ramped up later is one that spends
	/// its first months not protecting anything.
	pub fn dmarc_value(&self) -> String {
		format!(
			"v=DMARC1; p=reject; rua=mailto:dmarc@{}",
			self.report_domain()
		)
	}

	/// The TLS-RPT policy: report failures to negotiate authenticated TLS,
	/// which is the evidence MTA-STS `enforce` waits on.
	pub fn tls_rpt_value(&self) -> String {
		format!("v=TLSRPTv1; rua=mailto:tlsrpt@{}", self.report_domain())
	}

	/// The MTA-STS policy body this domain serves, ie the bytes published at
	/// [`WELL_KNOWN_PATH`](MtaStsPolicy::WELL_KNOWN_PATH) on
	/// [`MtaStsPolicy::host`].
	///
	/// Generated here rather than written by hand because the policy and the
	/// `_mta-sts` record are two halves of one statement: the record's id says
	/// "the policy changed", so a body edited without a re-apply leaves senders
	/// caching a policy the record no longer describes.
	pub fn mta_sts_policy_text(&self) -> String {
		self.mta_sts.policy_text(&[&self.mail_host])
	}

	/// The parameter this domain's sovereign DKIM private key lives at, ie
	/// `/beetmash/prod/dkim-stalwart-beetmash-com`. Composed here because three
	/// ends read it: the action that mints it, the record that publishes its
	/// public half, and the provision that hands it to the signing server.
	pub fn dkim_secret(&self) -> SecretRef {
		SecretRef::new(format!("dkim-{}", self.slug()))
	}

	/// The tofu variable the public half arrives as, ie
	/// `dkim_stalwart_beetmash_com`. Underscores rather than the slug's
	/// hyphens: `${var.a-b}` is a subtraction in HCL, not a name.
	pub fn dkim_public_key_variable(&self) -> Variable {
		Variable::param(
			self.slug()
				.replace('-', "_")
				.xmap(|slug| format!("dkim_{slug}")),
		)
	}

	/// The record `DKIM_SELECTOR` publishes at, ie
	/// `stalwart._domainkey.stalwart.beetmash.com`.
	pub fn dkim_record_name(&self) -> String {
		format!("{}._domainkey.{}", Self::DKIM_SELECTOR, self.domain)
	}

	/// The selector record's value, reading the public half out of the variable
	/// the [`EnsureDkimKey`] step resolved.
	///
	/// `h=sha256` is stated rather than defaulted, matching what Stalwart's own
	/// record generator emits for the same key, so the published record and the
	/// signature it validates are described identically.
	pub fn dkim_record_value(&self) -> String {
		format!(
			"v=DKIM1; k=rsa; h=sha256; p={}",
			self.dkim_public_key_variable().tf_var_ref()
		)
	}

	/// The terraform label suffix an alarm on `metric` takes, ie
	/// `reputation-bouncerate`.
	fn alarm_label(metric: &str) -> String {
		metric.replace('.', "-").to_lowercase()
	}

	/// The ARN of `topic` in `stack`'s region, composed against the account the
	/// apply is running in.
	fn topic_arn(stack: &ResolvedStack, topic: &SmolStr) -> String {
		format!(
			"arn:aws:sns:{}:${{data.aws_caller_identity.current.account_id}}:{topic}",
			stack.region()
		)
	}

	/// The ARN of [`alarms_topic`](Self::alarms_topic), ie where every arm's
	/// reputation alarms fire.
	fn alarms_topic_arn(&self, stack: &ResolvedStack) -> Option<String> {
		self.alarms_topic
			.as_ref()
			.map(|topic| Self::topic_arn(stack, topic))
	}

	/// A terraform label for this domain's `suffix` resource, distinct from
	/// every other domain's in the same stack.
	fn label(&self, suffix: &str) -> String {
		format!("{}--{suffix}", self.slug())
	}

	/// Reject a declaration that cannot provision: a bad member name, an alias
	/// pointing nowhere, or one localpart claimed twice.
	///
	/// Checked at config time rather than at provision time, since every one of
	/// these is a typo and the cheapest place to catch a typo is before any
	/// resource exists.
	pub fn validate(&self) -> Result {
		DnsProvider::validate_label(&self.slug(), "mail domain")?;
		for member in &self.members {
			member.validate()?;
		}
		let mut seen = HashSet::<SmolStr>::default();
		for localpart in self
			.mailboxes
			.iter()
			.map(Mailbox::localpart)
			.chain(self.aliases.iter().map(Alias::localpart))
		{
			if !seen.insert(localpart.clone()) {
				bevybail!(
					"'{localpart}@{}' is declared twice: a localpart is either a mailbox or an alias, never both",
					self.domain
				);
			}
		}
		for alias in &self.aliases {
			if !self
				.mailboxes
				.iter()
				.any(|mailbox| mailbox.localpart() == alias.target())
			{
				bevybail!(
					"alias '{}@{}' targets '{}', which is not a mailbox on this domain",
					alias.localpart(),
					self.domain,
					alias.target()
				);
			}
		}
		if let Some(catch_all) = &self.catch_all
			&& !self
				.mailboxes
				.iter()
				.any(|mailbox| mailbox.localpart() == catch_all)
		{
			bevybail!(
				"catch-all '{catch_all}@{}' is not a mailbox on this domain",
				self.domain
			);
		}
		Ok(())
	}
}

impl Block for MailDomainBlock {
	/// The name this block is known by is its domain.
	fn label(&self) -> &SmolStr { &self.domain }

	/// The sovereign selector's public key, resolved from the param
	/// [`EnsureDkimKey`](crate::mail::EnsureDkimKey) set. The filter mirrors
	/// that action's exactly: a domain whose records somebody else holds gets
	/// no key minted, so a variable here would fail resolution.
	///
	/// The relay-supplied variables are NOT here, because a block cannot see
	/// the component composed beside it: [`declare`](Self::declare) adds them.
	fn variables(&self) -> Vec<Variable> {
		match self.records.proves_identity() {
			true => vec![self.dkim_public_key_variable()],
			false => Vec::new(),
		}
	}
}

/// The [`DeployRender`] systems, registered by [`InfraPlugin`] beside the type
/// registration.
///
/// Bespoke rather than the generic pair, because a domain's emission depends on
/// the relay component composed beside it (or above it), which is a
/// cross-entity input.
impl MailDomainBlock {
	/// Contribute each domain's grants and variables, both of which the
	/// resolved relay adds to: a comail domain reads three records out of
	/// parameter store as `-var`s, and grants the metric namespace its
	/// deliverability poll publishes into.
	pub(crate) fn declare(
		mut scopes: AncestorQuery<&mut RenderScope>,
		blocks: Query<(Entity, &MailDomainBlock)>,
		relays: RelayQuery,
	) {
		for (entity, block) in blocks.iter() {
			if scopes.get_entity(entity).is_err() {
				continue;
			}
			let relay = relays.resolve(entity, Block::label(block));
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			match relay {
				Err(err) => scope.error(err),
				Ok(relay) => {
					let grants =
						block.grants(scope.stack()).xmap(|mut grants| {
							grants.extend(block.relay_grants(&relay));
							grants
						});
					let variables = block.variables().xmap(|mut variables| {
						variables.extend(block.relay_variables(&relay));
						variables
					});
					scope.declare(grants, variables);
				}
			}
		}
	}

	/// Emit each domain against its resolved relay, collecting per-entity
	/// errors rather than short-circuiting the rest.
	pub(crate) fn render(
		mut scopes: AncestorQuery<&mut RenderScope>,
		blocks: Query<(Entity, &MailDomainBlock)>,
		relays: RelayQuery,
	) {
		for (entity, block) in blocks.iter() {
			if scopes.get_entity(entity).is_err() {
				continue;
			}
			let relay = relays.resolve(entity, Block::label(block));
			let Ok(mut scope) = scopes.get_mut(entity) else {
				continue;
			};
			match relay {
				Err(err) => scope.error(err),
				Ok(relay) => {
					let (stack, deployment, config) = scope.ctx();
					if let Err(err) =
						block.emit(stack, deployment, config, &relay)
					{
						scope.error(bevyhow!(
							"MailDomainBlock '{}': {err}",
							Block::label(block)
						));
					}
				}
			}
		}
	}

	/// What a process running beside this domain needs, which for every mode
	/// but comail is nothing: only the deliverability poll writes anything at
	/// all, and it writes one CloudWatch namespace.
	fn relay_grants(&self, relay: &RelayMode) -> Vec<AccessGrant> {
		match relay {
			RelayMode::Comail(_) => vec![AccessGrant::read_write(
				ComailRelay::ACCESS_KIND,
				ComailRelay::METRIC_NAMESPACE,
			)],
			RelayMode::None | RelayMode::Ses(_) => Vec::new(),
		}
	}

	/// The variables the relay contributes: comail's selector stem and its two
	/// record values, each parked in parameter store by the operator and handed
	/// to the apply by [`ComailEnroll`].
	///
	/// The same shape as the sovereign key's, and for the same reason: the
	/// value is minted somewhere this deploy cannot reach, so the record reads
	/// it out of one parameter rather than restating it.
	fn relay_variables(&self, relay: &RelayMode) -> Vec<Variable> {
		match relay {
			RelayMode::Comail(_) if self.records.proves_identity() => {
				ComailRelay::dkim_variables(&self.slug())
					.into_iter()
					.map(|(variable, _)| variable)
					.collect()
			}
			_ => Vec::new(),
		}
	}
}

impl MailDomainBlock {
	/// Emit this domain against `relay`: the relay's own resources and identity
	/// records, the alarms that watch it, the dkim variable declarations, and,
	/// for the stage that owns the names, every dns record.
	fn emit(
		&self,
		stack: &ResolvedStack,
		deployment: &Deployment,
		config: &mut terra::Config,
		relay: &RelayMode,
	) -> Result {
		self.validate()?;
		// only the SES arm has resources at all: a comail domain is five
		// parameters and two TXT records, and a direct one is not even that.
		let identity = match relay {
			RelayMode::Ses(ses) => {
				let identity = self.emit_ses(stack, config)?;
				self.emit_ses_events(stack, config, ses)?;
				self.emit_ses_alarms(stack, config)?;
				Some(identity)
			}
			RelayMode::Comail(_) => {
				self.emit_comail_alarms(stack, config)?;
				None
			}
			RelayMode::None => None,
		};
		// declared whenever a variable resolves, not only when the record that
		// reads it is published: a `-var` for a variable the config never
		// declared fails the apply, and a stage outside `dns_stage` still
		// resolves them.
		for variable in self
			.variables()
			.into_iter()
			.chain(self.relay_variables(relay))
		{
			config.ensure_variable(
				variable.key().to_string(),
				variable.tf_declaration(),
			);
		}
		// the identity is stack-scoped and the records are not, so a stage that
		// does not own these names still gets a verified sender and publishes
		// nothing
		if !self.owns_names(stack) {
			warn!(
				"stage '{}' does not own '{}', so its records are not published",
				stack.stage(),
				self.domain
			);
			return Ok(());
		}
		match self.resolved_dns() {
			Some(dns) => {
				self.emit_identity_records(
					stack,
					config,
					&dns,
					relay,
					identity.as_ref(),
				)?;
				self.emit_delivery_records(
					stack, deployment, config, &dns, relay,
				)?;
			}
			// a domain that declares records but resolves no zone would apply
			// clean and publish nothing, which is the one failure a mail stack
			// cannot see: the identity exists, the deploy is green, and the
			// domain is undeliverable. Checked here rather than in `validate`
			// because which zone a launch has is not a property of the
			// declaration, and every non-deploy reader validates too.
			None if self.records.proves_identity() => bevybail!(
				"mail domain '{}' publishes records but no zone resolves: set CLOUDFLARE_ZONE_ID, `with_dns` a provider, or declare `records=\"None\"` for a domain whose records somebody else holds",
				self.domain
			),
			None => {}
		}
		Ok(())
	}
}

impl MailDomainBlock {
	/// The sending identity and the configuration set it reports through: one
	/// set per domain, so reputation, suppression and event streams are never
	/// pooled across domains that should be able to fail independently.
	///
	/// Returns the identity, whose computed DKIM tokens the `CNAME`s read.
	fn emit_ses(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result<ResourceDef<AwsSesv2EmailIdentityDetails>> {
		let set = ResourceDef::new_secondary(
			stack.resource_ident(self.label("ses-set")),
			AwsSesv2ConfigurationSetDetails {
				configuration_set_name: self.configuration_set_name().into(),
				reputation_options: Some(vec![
					AwsSesv2ConfigurationSetResourceBlockTypeReputationOptions {
						reputation_metrics_enabled: Some(true),
						..default()
					},
				]),
				sending_options: Some(vec![
					AwsSesv2ConfigurationSetResourceBlockTypeSendingOptions {
						sending_enabled: Some(true),
					},
				]),
				// per-set rather than account-wide: a domain that burns its
				// reputation must not suppress addresses for the others.
				suppression_options: Some(vec![
					AwsSesv2ConfigurationSetResourceBlockTypeSuppressionOptions {
						suppressed_reasons: Some(vec![
							"BOUNCE".into(),
							"COMPLAINT".into(),
						]),
					},
				]),
				..default()
			},
		);
		let identity = ResourceDef::new_secondary(
			stack.resource_ident(self.label("ses-identity")),
			AwsSesv2EmailIdentityDetails {
				email_identity: self.domain.clone(),
				configuration_set_name: Some(
					set.field_ref("configuration_set_name").into(),
				),
				dkim_signing_attributes: Some(vec![
					AwsSesv2EmailIdentityResourceBlockTypeDkimSigningAttributes {
						next_signing_key_length: Some(
							SesRelay::DKIM_KEY_LENGTH.into(),
						),
						..default()
					},
				]),
				..default()
			},
		);
		// the envelope sender's domain, so SPF authenticates a domain we own
		// rather than `amazonses.com`, and so aligns with the header From.
		// `USE_DEFAULT_VALUE` keeps mail flowing on a broken MAIL FROM record
		// instead of rejecting it; a `REJECT_MESSAGE` here turns a dns mistake
		// into an outage.
		let mail_from = ResourceDef::new_secondary(
			stack.resource_ident(self.label("ses-mail-from")),
			AwsSesv2EmailIdentityMailFromAttributesDetails {
				email_identity: identity.field_ref("email_identity").into(),
				mail_from_domain: Some(
					SesRelay::mail_from_domain(&self.domain).into(),
				),
				behavior_on_mx_failure: Some("USE_DEFAULT_VALUE".into()),
				..default()
			},
		);
		config
			.add_resource(&set)?
			.add_resource(&identity)?
			.add_resource(&mail_from)?;
		identity.xok()
	}

	/// The SES event destination that streams every bounce, complaint,
	/// rejection and delay onto the account's topic.
	///
	/// SES-only: it is a stream of SES event objects, so nothing but the SES
	/// arm can emit it, and it is named by [`SesRelay::events_topic`] rather
	/// than by the provider-neutral [`alarms_topic`](Self::alarms_topic).
	fn emit_ses_events(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		ses: &SesRelay,
	) -> Result {
		let Some(topic) = ses
			.events_topic()
			.as_ref()
			.map(|topic| Self::topic_arn(stack, topic))
		else {
			return Ok(());
		};
		// the account the apply runs in, since a topic name is not an address
		config.add_untyped_data_source(
			"aws_caller_identity",
			"current",
			&json!({}),
		)?;
		config.add_resource(&ResourceDef::new_secondary(
			stack.resource_ident(self.label("ses-events")),
			AwsSesv2ConfigurationSetEventDestinationDetails {
				configuration_set_name: self.configuration_set_name().into(),
				event_destination_name: SesRelay::EVENT_DESTINATION.into(),
				// the set is named by string rather than by field reference,
				// because the destination is a property OF the set rather than
				// a resource pointing at one; terraform sees no edge in a
				// string, so the order is stated
				depends_on: Some(vec![
					format!(
						"aws_sesv2_configuration_set.{}",
						stack.resource_ident(self.label("ses-set")).label()
					)
					.into(),
				]),
				event_destination: Some(vec![
					AwsSesv2ConfigurationSetEventDestinationResourceBlockTypeEventDestination {
						enabled: Some(true),
						matching_event_types: SesRelay::EVENT_TYPES
							.iter()
							.map(|event| SmolStr::from(*event))
							.collect(),
						sns_destination: Some(vec![
							EventDestinationResourceBlockTypeSnsDestination {
								topic_arn: topic.as_str().into(),
							},
						]),
						..default()
					},
				]),
				..default()
			},
		))?;
		Ok(())
	}

	/// The two reputation alarms on the `AWS/SES` metrics the configuration set
	/// publishes, which fire on the same numbers SES itself acts on and before
	/// it does.
	///
	/// `reputation_metrics_enabled` is not optional in [`emit_ses`](Self::emit_ses)
	/// because of these: without it the metric never appears and an alarm on it
	/// sits in `INSUFFICIENT_DATA` forever, reading exactly like a healthy one.
	fn emit_ses_alarms(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		self.emit_alarms(
			stack,
			config,
			SesRelay::METRIC_NAMESPACE,
			"ses:configuration-set",
			&self.configuration_set_name(),
			&[
				(
					"Reputation.BounceRate",
					SesRelay::BOUNCE_ALARM_RATE,
					"bounce rate: SES suspends a sender that stays here",
				),
				(
					"Reputation.ComplaintRate",
					SesRelay::COMPLAINT_ALARM_RATE,
					"complaint rate: recipients marking this domain as spam",
				),
			],
		)
	}

	/// The same two alarms on the metrics [`ComailDeliverability`] publishes,
	/// at thresholds under comail's own auto-pause so a warning beats the
	/// pause.
	///
	/// The metrics are written by that scheduled job and by nothing else, so a
	/// comail stack without it alarms on a metric that never appears, which
	/// reads exactly like a healthy one. That is why the example treats the job
	/// as part of what using comail means rather than as optional trim.
	fn emit_comail_alarms(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
	) -> Result {
		self.emit_alarms(
			stack,
			config,
			ComailRelay::METRIC_NAMESPACE,
			ComailRelay::METRIC_DIMENSION,
			&self.domain,
			&[
				(
					ComailRelay::BOUNCE_RATE_METRIC,
					ComailRelay::BOUNCE_ALARM_RATE,
					"bounce rate: comail pauses a domain that stays here",
				),
				(
					ComailRelay::COMPLAINT_RATE_METRIC,
					ComailRelay::COMPLAINT_ALARM_RATE,
					"complaint rate: recipients marking this domain as spam",
				),
			],
		)
	}

	/// One CloudWatch alarm per `(metric, threshold, description)`, all on
	/// `namespace` scoped by one `dimension`, firing on
	/// [`alarms_topic`](Self::alarms_topic).
	///
	/// Shared by both arms because a reputation alarm is the same statement
	/// whoever publishes the numbers: only the namespace, the dimension and the
	/// thresholds are the provider's.
	fn emit_alarms(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		namespace: &str,
		dimension: &str,
		dimension_value: &str,
		alarms: &[(&str, f64, &str)],
	) -> Result {
		let Some(topic) = self.alarms_topic_arn(stack) else {
			return Ok(());
		};
		// the account the apply runs in, since a topic name is not an address
		config.add_untyped_data_source(
			"aws_caller_identity",
			"current",
			&json!({}),
		)?;
		for (metric, threshold, description) in alarms {
			// UNTYPED, for the same reason the bucket's versioning is: a
			// reputation threshold is a fraction of a percent and the generated
			// binding types `threshold` as an integer, so 0.001 would round to
			// an alarm that never fires.
			config.add_untyped_resource(
				"aws_cloudwatch_metric_alarm",
				stack
					.resource_ident(self.label(&Self::alarm_label(metric)))
					.label(),
				&json!({
					"alarm_name": format!("{}-{metric}", self.slug()),
					"alarm_description": format!("{} {description}", self.domain),
					"namespace": namespace,
					"metric_name": metric,
					"dimensions": { dimension: dimension_value },
					"statistic": "Average",
					"comparison_operator": "GreaterThanThreshold",
					"threshold": threshold,
					"period": 3600,
					"evaluation_periods": 1,
					// no data means nothing was sent, which is not a fault
					"treat_missing_data": "notBreaching",
					"alarm_actions": [&topic],
					"ok_actions": [&topic],
				}),
			)?;
		}
		Ok(())
	}

	/// The records that prove the identity: the relay's own selectors, and the
	/// sovereign selector beside them.
	///
	/// Every one of them is selector- or subdomain-scoped, so they never
	/// collide with another provider's records on the same domain, which is
	/// what makes an [`IdentityOnly`](MailRecords::IdentityOnly) cutover
	/// possible at all.
	fn emit_identity_records(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		dns: &DnsProvider,
		relay: &RelayMode,
		identity: Option<&ResourceDef<AwsSesv2EmailIdentityDetails>>,
	) -> Result {
		if !self.records.proves_identity() {
			return Ok(());
		}
		// the sovereign selector, whose public half arrives as a variable: the
		// key is minted before the apply rather than by it, so the record and
		// the signing server read one value from one parameter. Published in
		// EVERY mode, which is the point of holding a key at all: nothing about
		// this domain's deliverability depends on a relay's.
		dns.emit_txt(
			stack,
			config,
			&self.label("dkim-sovereign"),
			&self.dkim_record_name(),
			&self.dkim_record_value(),
		)?;
		match relay {
			RelayMode::Ses(_) => {
				let identity = identity.ok_or_else(|| {
					bevyhow!(
						"the SES arm emits its identity before its records"
					)
				})?;
				self.emit_ses_identity_records(stack, config, dns, identity)?;
			}
			RelayMode::Comail(_) => {
				self.emit_comail_identity_records(stack, config, dns)?;
			}
			// direct delivery signs with the sovereign key and nothing else,
			// and has no envelope domain of its own to prove
			RelayMode::None => {}
		}
		Ok(())
	}

	/// The SES arm's identity records: the three Easy DKIM selectors read
	/// straight off the identity's computed tokens, and the custom MAIL FROM
	/// pair whose `MX` and SPF are what ALIGN the envelope sender with the
	/// header From.
	fn emit_ses_identity_records(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		dns: &DnsProvider,
		identity: &ResourceDef<AwsSesv2EmailIdentityDetails>,
	) -> Result {
		for index in 0..SesRelay::DKIM_TOKENS {
			// the token is only known after apply, so both halves of the record
			// are field-refs rather than the literal the console would show.
			let token = identity.field_ref(&format!(
				"dkim_signing_attributes[0].tokens[{index}]"
			));
			dns.emit_cname(
				stack,
				config,
				&self.label(&format!("dkim-{index}")),
				&format!("{token}._domainkey.{}", self.domain),
				&format!("{token}.dkim.amazonses.com"),
			)?;
		}
		let mail_from = SesRelay::mail_from_domain(&self.domain);
		dns.emit_mx(
			stack,
			config,
			&self.label("mail-from-mx"),
			&mail_from,
			Self::MX_PRIORITY,
			&SesRelay::feedback_host(&stack.region()),
		)?;
		dns.emit_txt(
			stack,
			config,
			&self.label("mail-from-spf"),
			&mail_from,
			SesRelay::SPF,
		)?;
		Ok(())
	}

	/// The comail arm's identity records: two `TXT` selectors off the
	/// date-based stem enrolment minted, RSA and Ed25519.
	///
	/// No custom MAIL FROM pair, and that is not an omission. Comail rewrites
	/// the envelope sender per recipient to a VERP address at its own domain,
	/// so a `bounce.<domain>` subdomain would authorise a sender that never
	/// sends and prove nothing; DMARC passes on the DKIM signature instead.
	///
	/// Both values and the stem arrive as variables, resolved from parameter
	/// store by [`ComailEnroll`], because enrolment mints them somewhere no
	/// deploy can reach.
	fn emit_comail_identity_records(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		dns: &DnsProvider,
	) -> Result {
		let slug = self.slug();
		for (label, name, value) in [
			(
				"comail-dkim-rsa",
				ComailRelay::dkim_rsa_record_name(&self.domain, &slug),
				ComailRelay::dkim_rsa_variable(&slug).tf_var_ref(),
			),
			(
				"comail-dkim-ed",
				ComailRelay::dkim_ed_record_name(&self.domain, &slug),
				ComailRelay::dkim_ed_variable(&slug).tf_var_ref(),
			),
		] {
			dns.emit_txt(stack, config, &self.label(label), &name, &value)?;
		}
		Ok(())
	}

	/// The records that hand this domain's mail to this stack: where to deliver
	/// it, who may send as it, what to do when neither checks out, and how a
	/// client finds the server. Every one of them is the dns specification's
	/// row for this domain, and nothing here names a domain other than the one
	/// declared.
	///
	/// Provider-neutral but for the SPF, which names whatever `relay` is.
	fn emit_delivery_records(
		&self,
		stack: &ResolvedStack,
		deployment: &Deployment,
		config: &mut terra::Config,
		dns: &DnsProvider,
		relay: &RelayMode,
	) -> Result {
		if !self.records.serves_mail() {
			return Ok(());
		}
		let domain = self.domain.as_str();
		dns.emit_mx(
			stack,
			config,
			&self.label("mx"),
			domain,
			Self::MX_PRIORITY,
			&self.mail_host,
		)?;
		// the one delivery record the relay decides: who may send as this
		// domain is exactly the question a relay answers, so `mx` (the box
		// itself), an `include:` of the relay, or nothing at all.
		dns.emit_txt(
			stack,
			config,
			&self.label("spf"),
			domain,
			&relay.spf_value(),
		)?;
		dns.emit_txt(
			stack,
			config,
			&self.label("dmarc"),
			&format!("_dmarc.{domain}"),
			&self.dmarc_value(),
		)?;
		dns.emit_txt(
			stack,
			config,
			&self.label("tls-rpt"),
			&format!("_smtp._tls.{domain}"),
			&self.tls_rpt_value(),
		)?;
		// the id changes with the deploy, so a policy edit is picked up by
		// senders on their next lookup rather than after `max_age` expires.
		dns.emit_txt(
			stack,
			config,
			&self.label("mta-sts"),
			&MtaStsPolicy::record_name(domain),
			&MtaStsPolicy::record_value(&MtaStsPolicy::policy_id(
				deployment.deploy_timestamp(),
			)),
		)?;
		for label in Self::AUTOCONFIG_LABELS {
			dns.emit_cname(
				stack,
				config,
				&self.label(label),
				&format!("{label}.{domain}"),
				&self.mail_host,
			)?;
		}
		for (service, port) in Self::SERVICES {
			dns.emit_srv(
				stack,
				config,
				&self.label(service),
				&format!("{service}.{domain}"),
				0,
				1,
				*port,
				&self.mail_host,
			)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use serde_json::Value;

	/// The staging domain as the plan declares it, which is also the fixture
	/// every assertion below reads. Never the apex: an apex mail record
	/// published by this stack before its cutover window would take live mail
	/// off a third party.
	fn staging() -> MailDomainBlock {
		MailDomainBlock::new("stalwart.beetmash.com", "mail.beetmash.com")
			.with_dns(DnsProvider::cloudflare(
				"stalwart.beetmash.com",
				"zone123",
			))
			.with_member(Member::new("pete"))
			.with_member(Member::new("info"))
			.with_mailbox(Mailbox::new("probe"))
			.with_role_aliases("pete", "info")
	}

	/// The publications domain, which shares the box and the zone but nothing
	/// else: its own identity, its own reputation, and replies welcomed into a
	/// catch-all.
	fn news() -> MailDomainBlock {
		MailDomainBlock::new("news.beetmash.com", "mail.beetmash.com")
			.with_dns(DnsProvider::cloudflare("news.beetmash.com", "zone123"))
			.with_report_domain("stalwart.beetmash.com")
			.with_mailbox(Mailbox::new("publications"))
			.with_alias(Alias::new("blog", "publications"))
			.with_catch_all("publications")
	}

	/// The Sydney stack every test renders against, ie the one the mail stack
	/// deploys into.
	fn sydney_stack() -> Stack {
		Stack::new("beet_infra").with_region(aws::region::AP_SOUTHEAST_2)
	}

	/// One domain and the relay composed beside it, ie what a declaration
	/// authors as `<MailDomainBlock domain=".." {SesRelay}/>`.
	type Relayed = (MailDomainBlock, RelayMode);

	/// `blocks` each relayed through SES, ie the arrangement every record set
	/// pinned below was written against.
	fn ses(blocks: &[MailDomainBlock]) -> Vec<Relayed> {
		relayed(blocks, RelayMode::Ses(SesRelay::default()))
	}

	/// `blocks` each relayed through `mode`.
	fn relayed(blocks: &[MailDomainBlock], mode: RelayMode) -> Vec<Relayed> {
		blocks
			.iter()
			.cloned()
			.map(|block| (block, mode.clone()))
			.collect()
	}

	/// The config `blocks` emit against a Sydney stack, each with its relay
	/// composed on the same entity.
	fn build_config(blocks: &[Relayed]) -> terra::Config {
		let blocks = blocks.to_vec();
		let (scope, _dir) =
			RenderScope::test_render_stack(sydney_stack(), |parent| {
				for (block, mode) in blocks {
					mode.insert(&mut parent.spawn(block));
				}
			});
		scope.finish().unwrap().2
	}

	/// Every dns record `blocks` render, as `(type, name)`, with any terraform
	/// interpolation in the name collapsed to `<dkim-N>`. The dns specification
	/// is a table of exactly these two columns, so the tests read as the table
	/// does rather than as terraform.
	fn records(blocks: &[Relayed]) -> Vec<(String, String)> {
		record_values(blocks)
			.into_iter()
			.map(|(record_type, name, _)| (record_type, collapse_refs(&name)))
			.collect()
	}

	/// Collapse a `${aws_sesv2_email_identity.<label>.dkim_signing_attributes[0]
	/// .tokens[N]}` interpolation to `<dkim-N>`, so a pinned record name reads
	/// as the selector it is rather than as the resource address it composes
	/// from (which the stack identity, not the dns spec, decides).
	fn collapse_refs(name: &str) -> String {
		let (Some(start), Some(end)) = (name.find("${"), name.find('}')) else {
			return name.to_string();
		};
		let index = name[start..end]
			.rsplit_once("tokens[")
			.map(|(_, rest)| rest.trim_end_matches(']'))
			.unwrap_or_default();
		format!("<dkim-{index}>{}", &name[end + 1..])
	}

	/// As [`records`], with each record's content (`data.target` for the `SRV`s,
	/// which carry no `content`).
	fn record_values(blocks: &[Relayed]) -> Vec<(String, String, String)> {
		let json = build_config(blocks).to_json().into_json();
		let Some(Value::Object(records)) = json
			.get("resource")
			.and_then(|it| it.get("cloudflare_dns_record"))
		else {
			return Vec::new();
		};
		let mut records = records
			.values()
			.map(|record| {
				let field = |key: &str| {
					record
						.get(key)
						.and_then(Value::as_str)
						.unwrap_or_default()
						.to_string()
				};
				let content = match field("content").is_empty() {
					false => field("content"),
					true => record
						.get("data")
						.and_then(|data| data.get("target"))
						.and_then(Value::as_str)
						.unwrap_or_default()
						.to_string(),
				};
				(field("type"), field("name"), content)
			})
			.collect::<Vec<_>>();
		records.sort();
		records
	}

	/// The dns specification IS the spec, so the staging domain's record set is
	/// pinned in full: a record silently dropped from this list is mail that
	/// stops arriving, and one silently added is a name this stack claimed
	/// without saying so.
	#[beet_core::test]
	fn staging_domain_emits_the_specification() {
		records(&ses(&[staging()]))
			.into_iter()
			.map(|(record_type, name)| format!("{record_type} {name}"))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"CNAME <dkim-0>._domainkey.stalwart.beetmash.com",
				"CNAME <dkim-1>._domainkey.stalwart.beetmash.com",
				"CNAME <dkim-2>._domainkey.stalwart.beetmash.com",
				"CNAME autoconfig.stalwart.beetmash.com",
				"CNAME autodiscover.stalwart.beetmash.com",
				"MX bounce.stalwart.beetmash.com",
				"MX stalwart.beetmash.com",
				"SRV _imaps._tcp.stalwart.beetmash.com",
				"SRV _jmap._tcp.stalwart.beetmash.com",
				"SRV _submissions._tcp.stalwart.beetmash.com",
				"TXT _dmarc.stalwart.beetmash.com",
				"TXT _mta-sts.stalwart.beetmash.com",
				"TXT _smtp._tls.stalwart.beetmash.com",
				"TXT bounce.stalwart.beetmash.com",
				"TXT stalwart._domainkey.stalwart.beetmash.com",
				"TXT stalwart.beetmash.com",
			]);
	}

	/// The sovereign selector reads its key out of the variable rather than
	/// carrying a literal, which is what keeps the published record and the
	/// key the server signs with one value from one parameter.
	#[beet_core::test]
	fn the_sovereign_selector_reads_the_minted_key() {
		let config = build_config(&ses(&[staging()]));
		record_values(&ses(&[staging()]))
			.into_iter()
			.find(|(_, name, _)| name.starts_with("stalwart._domainkey."))
			.unwrap()
			.2
			.as_str()
			.xpect_eq(
				"v=DKIM1; k=rsa; h=sha256; p=${var.dkim_stalwart_beetmash_com}",
			);
		// declared, so `plan` and `destroy` (which pass no `-var`) still parse
		config.to_json().into_json()["variable"]["dkim_stalwart_beetmash_com"]
			.is_object()
			.xpect_true();
	}

	/// Nothing consumes a sending domain's events until a topic is named, and a
	/// reputation alarm with nowhere to fire is worse than none: it reads as
	/// monitoring. So each arrives with the topic that names it, and neither
	/// without.
	#[beet_core::test]
	fn events_and_alarms_arrive_with_their_topics() {
		build_config(&ses(&[staging()]))
			.to_json_string()
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("aws_cloudwatch_metric_alarm")
			.xnot()
			.xpect_contains("aws_sesv2_configuration_set_event_destination");

		let json = build_config(&[(
			staging().with_alarms_topic("beetmash-ses-events"),
			RelayMode::Ses(
				SesRelay::default().with_events_topic("beetmash-ses-events"),
			),
		)])
		.to_json()
		.into_json();
		let alarms = json["resource"]["aws_cloudwatch_metric_alarm"]
			.as_object()
			.unwrap()
			.values()
			.collect::<Vec<_>>();
		alarms.len().xpect_eq(2);
		for alarm in alarms {
			alarm["namespace"].as_str().unwrap().xpect_eq("AWS/SES");
			alarm["dimensions"]["ses:configuration-set"]
				.as_str()
				.unwrap()
				.xpect_eq("stalwart-beetmash-com");
			// a fraction of a percent, which an integer threshold would round
			// away into an alarm that never fires
			alarm["threshold"].as_f64().unwrap().xpect_greater_than(0.);
			alarm["alarm_actions"][0]
				.as_str()
				.unwrap()
				.xpect_contains("beetmash-ses-events");
		}
		json["resource"]["aws_sesv2_configuration_set_event_destination"]
			.as_object()
			.unwrap()
			.values()
			.next()
			.unwrap()["event_destination"][0]["matching_event_types"]
			.as_array()
			.unwrap()
			.len()
			.xpect_eq(SesRelay::EVENT_TYPES.len());
	}

	/// Exactly one SPF record per NAME, which is the invariant that makes the
	/// staging domain safe beside a third party's apex: two SPF records at ONE
	/// name is a permerror (every check fails), while two records at different
	/// names are simply two domains.
	#[beet_core::test]
	fn one_spf_record_per_name() {
		let mut names = HashMap::<String, usize>::default();
		for (_, name, _) in record_values(&ses(&[staging(), news()]))
			.into_iter()
			.filter(|(record_type, _, content)| {
				record_type == "TXT" && content.starts_with("v=spf1")
			}) {
			*names.entry(name).or_default() += 1;
		}
		// the two served domains and their two envelope-sender subdomains
		names.len().xpect_eq(4);
		names.values().copied().max().unwrap().xpect_eq(1);
	}

	/// No block whose domain is not the apex may emit an apex record. Fastmail
	/// serves `@beetmash.com` for the whole build, so an apex `MX`, SPF or
	/// `_dmarc` published from a staging instance is not a shortcut, it is mail
	/// taken off a live provider.
	#[beet_core::test]
	fn staging_never_touches_the_apex() {
		for (_, name) in records(&ses(&[staging(), news()])) {
			// a record AT the apex, rather than one merely under it
			["beetmash.com", "_dmarc.beetmash.com"]
				.contains(&name.as_str())
				.xpect_false();
			name.ends_with(".beetmash.com").xpect_true();
		}
	}

	/// An identity prepared ahead of its cutover verifies and signs, and moves
	/// no mail: the selector `CNAME`s and the `bounce.` pair are namespaced, so
	/// they sit beside whoever is still serving the domain.
	#[beet_core::test]
	fn identity_only_publishes_nothing_that_moves_mail() {
		let apex = MailDomainBlock::new("beetmash.com", "mail.beetmash.com")
			.with_records(MailRecords::IdentityOnly)
			.with_dns(DnsProvider::cloudflare("beetmash.com", "zone123"));
		records(&ses(&[apex]))
			.into_iter()
			.map(|(record_type, name)| format!("{record_type} {name}"))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"CNAME <dkim-0>._domainkey.beetmash.com",
				"CNAME <dkim-1>._domainkey.beetmash.com",
				"CNAME <dkim-2>._domainkey.beetmash.com",
				// the envelope-sender pair, under `bounce.` and never at the
				// apex itself, so the incumbent's own SPF is untouched
				"MX bounce.beetmash.com",
				"TXT bounce.beetmash.com",
				// the sovereign selector, which is a selector like any other:
				// it authorises one more signature and displaces nobody
				"TXT stalwart._domainkey.beetmash.com",
			]);
	}

	/// A mail name is not stack-composed: `stalwart.beetmash.com` is the real
	/// name of real mail, so a second stage deploying the same declaration
	/// would publish a SECOND record at each name and receivers would
	/// round-robin between the live box and whatever the experiment stood up.
	/// The stage that owns the names is the only one that may publish them.
	#[beet_core::test]
	fn only_the_owning_stage_publishes_the_records() {
		let records_in = |stage: &str| {
			let (scope, _dir) = RenderScope::test_render_stack(
				Stack::new("beet_infra").with_stage(stage),
				|parent| {
					parent.spawn((
						staging().with_dns_stage("prod"),
						SesRelay::default(),
					));
				},
			);
			scope.finish().unwrap().2.to_json().into_json()
		};
		records_in("prod")["resource"]["cloudflare_dns_record"]
			.is_object()
			.xpect_true();
		let drill = records_in("drill");
		drill["resource"]["cloudflare_dns_record"]
			.is_null()
			.xpect_true();
		// ..while the sending identity, which IS stack-scoped, still exists
		drill["resource"]["aws_sesv2_email_identity"]
			.is_object()
			.xpect_true();
	}

	/// The block must HAND its dkim variable to the apply, not merely declare
	/// it in the config: a declared-only variable resolves to its empty
	/// default, and `p=` empty is the wire form of a REVOKED selector. This is
	/// exactly what the first phase-8 deploy published.
	#[beet_core::test]
	fn the_apply_resolves_the_dkim_variable() {
		staging()
			.variables()
			.into_iter()
			.map(|variable| variable.key().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["dkim_stalwart_beetmash_com".to_string()]);
		// no key is minted for a domain whose records somebody else holds, so
		// a variable here would fail resolution rather than default
		MailDomainBlock::new("beetmash.com", "mail.beetmash.com")
			.with_records(MailRecords::None)
			.variables()
			.len()
			.xpect_eq(0);
		// ..and a stage outside `dns_stage` publishes no record but still
		// declares the variable, since the apply passes the `-var` regardless
		// and an undeclared one fails the apply
		let (scope, _dir) = RenderScope::test_render_stack(
			Stack::new("beet_infra").with_stage("drill"),
			|parent| {
				parent.spawn((
					staging().with_dns_stage("prod"),
					SesRelay::default(),
				));
			},
		);
		scope.finish().unwrap().2.to_json().into_json()["variable"]
			["dkim_stalwart_beetmash_com"]
			.is_object()
			.xpect_true();
	}

	/// A block declaring [`MailRecords::None`] declares the identity and stays
	/// out of dns entirely, which is how a domain whose records somebody else
	/// holds still gets a verified sending identity.
	#[beet_core::test]
	fn records_none_emits_no_records() {
		let block = MailDomainBlock::new("beetmash.com", "mail.beetmash.com")
			.with_records(MailRecords::None);
		records(&ses(&[block.clone()])).xpect_eq(Vec::new());
		build_config(&ses(&[block]))
			.to_json_string()
			.unwrap()
			.xpect_contains("aws_sesv2_email_identity");
	}

	/// The DKIM records read the selectors off the identity's computed tokens
	/// rather than restating the ones a console showed, so a rotated key is a
	/// re-apply and not a hunt through the zone.
	#[beet_core::test]
	fn dkim_records_reference_the_identity_outputs() {
		let (_, _, content) = record_values(&ses(&[staging()]))
			.into_iter()
			.find(|(_, name, _)| name.contains("._domainkey."))
			.unwrap();
		content
			.as_str()
			.xpect_contains(".dkim_signing_attributes[0].tokens[0]}")
			.xpect_contains(".dkim.amazonses.com");
	}

	/// The set name is fixed by the account's existing resources so terraform
	/// can adopt rather than duplicate them, which means it is NOT
	/// stack-composed: `stalwart-beetmash-com`, not `beet-infra--dev--...`.
	#[beet_core::test]
	fn configuration_set_is_named_from_the_domain() {
		build_config(&ses(&[staging(), news()]))
			.to_json_string()
			.unwrap()
			.xpect_contains(
				"\"configuration_set_name\":\"stalwart-beetmash-com\"",
			)
			.xpect_contains("\"configuration_set_name\":\"news-beetmash-com\"");
	}

	/// The feedback host is regional and the region belongs to the stack, so a
	/// relocated stack moves it without the block naming a region at all.
	#[beet_core::test]
	fn mail_from_mx_follows_the_stack_region() {
		record_values(&ses(&[staging()]))
			.into_iter()
			.find(|(record_type, name, _)| {
				record_type == "MX" && name.starts_with("bounce.")
			})
			.unwrap()
			.2
			.as_str()
			.xpect_eq("feedback-smtp.ap-southeast-2.amazonses.com");
	}

	/// Reports have to reach a mailbox that exists. While a domain is being
	/// commissioned that is another domain's, so DMARC and TLS-RPT both follow
	/// the declared report domain rather than assuming their own.
	#[beet_core::test]
	fn reports_are_addressed_to_the_report_domain() {
		let values = record_values(&ses(&[news()]));
		let content = |name: &str| {
			values
				.iter()
				.find(|(_, record_name, _)| record_name == name)
				.unwrap()
				.2
				.clone()
		};
		content("_dmarc.news.beetmash.com").as_str().xpect_eq(
			"v=DMARC1; p=reject; rua=mailto:dmarc@stalwart.beetmash.com",
		);
		content("_smtp._tls.news.beetmash.com")
			.as_str()
			.xpect_eq("v=TLSRPTv1; rua=mailto:tlsrpt@stalwart.beetmash.com");
	}


	/// The policy body names the box, not the domain: a sender matches the
	/// certificate the box presents, which is the same certificate whichever
	/// domain the mail was addressed to.
	#[beet_core::test]
	fn mta_sts_policy_names_the_mail_host() {
		staging()
			.mta_sts_policy_text()
			.as_str()
			.xpect_eq("version: STSv1\r\nmode: testing\r\nmx: mail.beetmash.com\r\nmax_age: 604800\r\n");
		// ..and the record advertising it is published for the domain
		records(&ses(&[staging()]))
			.contains(&(
				"TXT".to_string(),
				"_mta-sts.stalwart.beetmash.com".to_string(),
			))
			.xpect_true();
	}

	/// A typo in a declaration is caught before any resource exists, since the
	/// alternative is discovering it against a half-provisioned mail server.
	#[beet_core::test]
	fn invalid_declarations_fail_at_config_time() {
		let block = |block: MailDomainBlock| {
			let (scope, _dir) = RenderScope::test_render(|parent| {
				parent.spawn(block);
			});
			scope.finish().unwrap_err().to_string()
		};
		let base = || {
			MailDomainBlock::new("stalwart.beetmash.com", "mail.beetmash.com")
		};
		block(base().with_alias(Alias::new("postmaster", "nobody")))
			.xpect_contains("not a mailbox on this domain");
		block(
			base()
				.with_mailbox(Mailbox::new("pete"))
				.with_alias(Alias::new("pete", "pete")),
		)
		.xpect_contains("declared twice");
		block(base().with_member(Member::new("mail")))
			.xpect_contains("reserved hostname");
		block(base().with_catch_all("publications"))
			.xpect_contains("catch-all");
	}

	/// A comail domain declares nothing in AWS at all: no identity, no
	/// configuration set, no IAM. What it publishes is the two selectors
	/// enrolment minted and an SPF that includes the relay, both of which sit
	/// beside the sovereign selector rather than replacing it.
	#[beet_core::test]
	fn a_comail_domain_publishes_two_selectors_and_no_aws() {
		let blocks =
			relayed(&[news()], RelayMode::Comail(ComailRelay::default()));
		build_config(&blocks)
			.to_json_string()
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("aws_sesv2")
			.xnot()
			.xpect_contains("aws_iam")
			.xnot()
			.xpect_contains("amazonses.com");
		let names = records(&blocks)
			.into_iter()
			.map(|(record_type, name)| format!("{record_type} {name}"))
			.collect::<Vec<_>>();
		// the selector stem is a variable, so the interpolation collapses to
		// `<dkim->` the way a computed SES token does
		names
			.iter()
			.filter(|name| name.contains("_domainkey"))
			.cloned()
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"TXT <dkim->e._domainkey.news.beetmash.com".to_string(),
				"TXT <dkim->r._domainkey.news.beetmash.com".to_string(),
				"TXT stalwart._domainkey.news.beetmash.com".to_string(),
			]);
		// and NO envelope-sender subdomain: comail rewrites MAIL FROM to a VERP
		// address at its own domain, so a `bounce.` pair would authorise a
		// sender that never sends
		names
			.iter()
			.any(|name| name.contains("bounce."))
			.xpect_false();
		record_values(&blocks)
			.into_iter()
			.find(|(record_type, name, content)| {
				record_type == "TXT"
					&& name == "news.beetmash.com"
					&& content.starts_with("v=spf1")
			})
			.unwrap()
			.2
			.as_str()
			.xpect_eq("v=spf1 include:atmos.email ~all");
	}

	/// A domain with no relay delivers straight to each recipient's MX, so its
	/// SPF authorises the box by the `mx` mechanism (the box IS the domain's
	/// MX target) and nothing else, and it holds one selector: its own.
	#[beet_core::test]
	fn a_direct_domain_authorises_its_own_mx() {
		let blocks = relayed(&[news()], RelayMode::None);
		build_config(&blocks)
			.to_json_string()
			.unwrap()
			.as_str()
			.xnot()
			.xpect_contains("aws_sesv2")
			.xnot()
			.xpect_contains("amazonses.com")
			.xnot()
			.xpect_contains("atmos.email");
		records(&blocks)
			.into_iter()
			.map(|(record_type, name)| format!("{record_type} {name}"))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"CNAME autoconfig.news.beetmash.com",
				"CNAME autodiscover.news.beetmash.com",
				"MX news.beetmash.com",
				"SRV _imaps._tcp.news.beetmash.com",
				"SRV _jmap._tcp.news.beetmash.com",
				"SRV _submissions._tcp.news.beetmash.com",
				"TXT _dmarc.news.beetmash.com",
				"TXT _mta-sts.news.beetmash.com",
				"TXT _smtp._tls.news.beetmash.com",
				"TXT news.beetmash.com",
				"TXT stalwart._domainkey.news.beetmash.com",
			]);
		record_values(&blocks)
			.into_iter()
			.find(|(_, name, content)| {
				name == "news.beetmash.com" && content.starts_with("v=spf1")
			})
			.unwrap()
			.2
			.as_str()
			.xpect_eq("v=spf1 mx -all");
	}

	/// The comail arm alarms on the metrics its poll publishes, under the rates
	/// comail itself pauses a domain at, and fires on the provider-neutral
	/// topic rather than on a SES event stream it does not have.
	#[beet_core::test]
	fn comail_alarms_sit_below_the_auto_pause() {
		let json = build_config(&[(
			news().with_alarms_topic("beetmash-mail-alarms"),
			RelayMode::Comail(ComailRelay::default()),
		)])
		.to_json()
		.into_json();
		let alarms = json["resource"]["aws_cloudwatch_metric_alarm"]
			.as_object()
			.unwrap()
			.values()
			.collect::<Vec<_>>();
		alarms.len().xpect_eq(2);
		for alarm in alarms {
			alarm["namespace"].as_str().unwrap().xpect_eq("Comail");
			alarm["dimensions"]["Domain"]
				.as_str()
				.unwrap()
				.xpect_eq("news.beetmash.com");
			alarm["alarm_actions"][0]
				.as_str()
				.unwrap()
				.xpect_contains("beetmash-mail-alarms");
			let threshold = alarm["threshold"].as_f64().unwrap();
			threshold.xpect_greater_than(0.);
			threshold.xpect_less_than(ComailRelay::PAUSE_BOUNCE_RATE);
		}
		// no SES event stream exists to point at, and none is emitted
		json["resource"]["aws_sesv2_configuration_set_event_destination"]
			.is_null()
			.xpect_true();
	}

	/// A comail domain hands the apply the three values enrolment minted, and
	/// declares them, for the same reason the sovereign key is handed over: a
	/// variable that only defaults resolves to empty, and an empty selector
	/// record is a REVOKED one.
	#[beet_core::test]
	fn a_comail_domain_declares_its_enrolled_records() {
		let blocks =
			relayed(&[news()], RelayMode::Comail(ComailRelay::default()));
		let variables = build_config(&blocks).to_json().into_json()["variable"]
			.as_object()
			.unwrap()
			.keys()
			.cloned()
			.collect::<HashSet<_>>();
		for suffix in ["dkim_selector", "dkim_rsa", "dkim_ed"] {
			variables
				.contains(&format!("comail_{suffix}_news_beetmash_com"))
				.xpect_true();
		}
		// ..and the sovereign key's, which every mode keeps
		variables.contains("dkim_news_beetmash_com").xpect_true();
	}
}
