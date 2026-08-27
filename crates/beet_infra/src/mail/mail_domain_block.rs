use crate::bindings::*;
use crate::mail::*;
use crate::prelude::*;
use crate::terra::ResourceDef;
use beet_core::prelude::*;

/// Which of a mail domain's records a [`MailDomainBlock`] publishes.
///
/// A domain's records fall into two groups with very different blast radius,
/// and the distinction is what makes a cutover a short window rather than a
/// second build.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize,
)]
pub enum MailRecords {
	/// Everything: this stack serves the domain's mail.
	#[default]
	All,
	/// Only the records that prove the SES identity: its DKIM selectors and its
	/// custom MAIL FROM pair. Both are namespaced (by selector token, and under
	/// a `bounce.` subdomain), so they coexist with a third party still serving
	/// the domain's mail. This is a cutover prepared ahead of its window: the
	/// identity verifies, and not one message changes hands.
	IdentityOnly,
	/// None. The SES identity exists and its records are published elsewhere.
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

/// One mail domain, end to end: the SES sending identity that signs its
/// outbound mail, and every DNS record that makes the domain deliverable and
/// discoverable.
///
/// The domain is the only thing declared. Every record name, every SES resource
/// name and the MTA-STS host all derive from it, so serving a second domain is
/// a second instance rather than an edit, and the eventual cutover from a
/// staging domain to the apex is one more instance beside the others.
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
#[derive(Debug, Clone, Get, SetWith, Serialize, Deserialize, Component)]
#[component(immutable, on_add = ErasedBlock::on_add::<MailDomainBlock>)]
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
	/// the SES identity and nothing else, ie a domain whose records are held by
	/// somebody else.
	#[get(skip)]
	#[set_with(unwrap_option)]
	dns: Option<DnsProvider>,
	/// The domain the DMARC aggregate and TLS-RPT report addresses live on.
	/// Reports have to reach a mailbox that exists, which while a domain is
	/// being commissioned is not that domain's own.
	report_domain: SmolStr,
	/// See [`MtaStsPolicy`].
	mta_sts: MtaStsPolicy,
	/// The mailboxes this domain stores mail for, read by `StalwartProvision`.
	#[set_with(skip)]
	mailboxes: Vec<Mailbox>,
	/// The localparts that redirect into those mailboxes, read by
	/// `StalwartProvision`.
	#[set_with(skip)]
	aliases: Vec<Alias>,
	/// The members whose identities this domain carries. Their atproto handles
	/// are published when [`handle_domain`](Self::handle_domain) names where.
	#[set_with(skip)]
	members: Vec<Member>,
	/// The mailbox every otherwise unmatched address on this domain delivers
	/// to. A publication domain wants one (replies to a newsletter are welcome
	/// and unpredictable); a person's domain does not (a catch-all is a spam
	/// trap that accepts).
	#[set_with(unwrap_option, into)]
	catch_all: Option<SmolStr>,
	/// The domain member handles hang off, ie the `beetmash.com` in
	/// `_atproto.pete.beetmash.com`. Handles are an organisation-level name and
	/// several mail domains share one organisation, so exactly one block names
	/// it and the rest leave it unset rather than each publishing the same
	/// records.
	#[set_with(unwrap_option, into)]
	handle_domain: Option<SmolStr>,
}

impl MailDomainBlock {
	/// The SPF policy every domain this stack sends for publishes: SES is the
	/// only authorised sender, and `-all` (hard fail) rather than `~all`,
	/// because a soft fail asks receivers to accept forgeries and think about
	/// it.
	pub const SPF: &'static str = "v=spf1 include:amazonses.com -all";

	/// The subdomain SES's custom MAIL FROM uses, ie the envelope sender's
	/// domain. Its own `MX` and SPF are what make SPF *aligned* with the header
	/// From, which is one of the two ways to pass DMARC.
	pub const MAIL_FROM_LABEL: &'static str = "bounce";

	/// The single-MX preference. Nothing weighs against anything, so the value
	/// is convention rather than a choice.
	pub const MX_PRIORITY: u16 = 10;

	/// Easy DKIM publishes three rotating selectors, so three `CNAME`s per
	/// identity. Fixed by SES.
	pub const DKIM_TOKENS: usize = 3;

	/// The sesv2 key-length enum. Note the `_BIT` suffix: the v1 API and most
	/// documentation say `RSA_2048`, which sesv2 rejects.
	pub const DKIM_KEY_LENGTH: &'static str = "RSA_2048_BIT";

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
			report_domain: domain.clone(),
			mta_sts: default(),
			mailboxes: Vec::new(),
			aliases: Vec::new(),
			members: Vec::new(),
			catch_all: None,
			handle_domain: None,
			domain,
		}
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
	/// terraform labels, and the SES configuration set name.
	pub fn slug(&self) -> String { self.domain.replace('.', "-") }

	/// The SES configuration set this domain sends through. Named from the
	/// domain and NOT stack-composed, so an account's sets read as the domains
	/// they belong to and terraform can adopt one made by hand.
	pub fn configuration_set_name(&self) -> String { self.slug() }

	/// The envelope-sender domain, ie `bounce.stalwart.beetmash.com`.
	pub fn mail_from_domain(&self) -> String {
		format!("{}.{}", Self::MAIL_FROM_LABEL, self.domain)
	}

	/// The SES bounce/complaint feedback host for `region`, the `MX` the
	/// [`mail from domain`](Self::mail_from_domain) points at.
	pub fn feedback_host(region: &str) -> String {
		format!("feedback-smtp.{region}.amazonses.com")
	}

	/// The DMARC policy: reject outright, and report. `p=reject` from the first
	/// deploy is deliberate, since a policy ramped up later is one that spends
	/// its first months not protecting anything.
	pub fn dmarc_value(&self) -> String {
		format!(
			"v=DMARC1; p=reject; rua=mailto:dmarc@{}",
			self.report_domain
		)
	}

	/// The TLS-RPT policy: report failures to negotiate authenticated TLS,
	/// which is the evidence MTA-STS `enforce` waits on.
	pub fn tls_rpt_value(&self) -> String {
		format!("v=TLSRPTv1; rua=mailto:tlsrpt@{}", self.report_domain)
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
		validate_dns_label(&self.slug(), "mail domain")?;
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
	fn apply_to_config(
		&self,
		_entity: &EntityRef,
		stack: &ResolvedStack,
		deployment: &Deployment,
		_access: &AccessGrants,
		config: &mut terra::Config,
	) -> Result {
		self.validate()?;
		let identity = self.emit_ses(stack, config)?;
		if let Some(dns) = &self.dns {
			self.emit_identity_records(stack, config, dns, &identity)?;
			self.emit_delivery_records(stack, deployment, config, dns)?;
			self.emit_handle_records(stack, config, dns)?;
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
							Self::DKIM_KEY_LENGTH.into(),
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
				mail_from_domain: Some(self.mail_from_domain().into()),
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

	/// The records that prove the identity: the three Easy DKIM selectors read
	/// straight off the identity's computed tokens, and the custom MAIL FROM
	/// pair. Selector- and subdomain-scoped, so they never collide with another
	/// provider's records on the same domain.
	fn emit_identity_records(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		dns: &DnsProvider,
		identity: &ResourceDef<AwsSesv2EmailIdentityDetails>,
	) -> Result {
		if !self.records.proves_identity() {
			return Ok(());
		}
		for index in 0..Self::DKIM_TOKENS {
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
		let mail_from = self.mail_from_domain();
		dns.emit_mx(
			stack,
			config,
			&self.label("mail-from-mx"),
			&mail_from,
			Self::MX_PRIORITY,
			&Self::feedback_host(&stack.region()),
		)?;
		dns.emit_txt(
			stack,
			config,
			&self.label("mail-from-spf"),
			&mail_from,
			Self::SPF,
		)?;
		Ok(())
	}

	/// The records that hand this domain's mail to this stack: where to deliver
	/// it, who may send as it, what to do when neither checks out, and how a
	/// client finds the server. Every one of them is the dns specification's
	/// row for this domain, and nothing here names a domain other than the one
	/// declared.
	fn emit_delivery_records(
		&self,
		stack: &ResolvedStack,
		deployment: &Deployment,
		config: &mut terra::Config,
		dns: &DnsProvider,
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
		dns.emit_txt(stack, config, &self.label("spf"), domain, Self::SPF)?;
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

	/// The atproto handle of every member carrying a DID. Not a mail record: it
	/// rides here because a member declaration is already the one place an
	/// identity is described, and splitting it would mean declaring the same
	/// people twice.
	fn emit_handle_records(
		&self,
		stack: &ResolvedStack,
		config: &mut terra::Config,
		dns: &DnsProvider,
	) -> Result {
		let Some(handle_domain) = &self.handle_domain else {
			return Ok(());
		};
		for member in self.members.iter().filter(|it| it.did().is_some()) {
			let did = member.did().as_ref().unwrap();
			dns.emit_txt(
				stack,
				config,
				&self.label(&format!("atproto-{}", member.name())),
				&member.handle_record_name(handle_domain),
				&format!("did={did}"),
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
			.with_member(
				Member::new("pete").with_did("did:plc:examplepetehandle"),
			)
			.with_member(Member::new("info"))
			.with_mailbox(Mailbox::new("probe"))
			.with_role_aliases("pete", "info")
			.with_handle_domain("beetmash.com")
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

	/// The config `blocks` emit against a Sydney stack, ie the one the mail
	/// stack deploys into.
	fn build_config(blocks: &[MailDomainBlock]) -> terra::Config {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let stack = stack.with_region(aws::region::AP_SOUTHEAST_2);
		let mut config = deployment.create_config(&stack);
		let mut world = World::new();
		for block in blocks {
			block
				.apply_to_config(
					&world.spawn(()).as_readonly(),
					&stack,
					&deployment,
					&default(),
					&mut config,
				)
				.unwrap();
		}
		config
	}

	/// Every dns record `blocks` render, as `(type, name)`, with any terraform
	/// interpolation in the name collapsed to `<dkim-N>`. The dns specification
	/// is a table of exactly these two columns, so the tests read as the table
	/// does rather than as terraform.
	fn records(blocks: &[MailDomainBlock]) -> Vec<(String, String)> {
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
	fn record_values(
		blocks: &[MailDomainBlock],
	) -> Vec<(String, String, String)> {
		let json = build_config(blocks).to_json();
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
		records(&[staging()])
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
				"TXT _atproto.pete.beetmash.com",
				"TXT _dmarc.stalwart.beetmash.com",
				"TXT _mta-sts.stalwart.beetmash.com",
				"TXT _smtp._tls.stalwart.beetmash.com",
				"TXT bounce.stalwart.beetmash.com",
				"TXT stalwart.beetmash.com",
			]);
	}

	/// Exactly one SPF record per NAME, which is the invariant that makes the
	/// staging domain safe beside a third party's apex: two SPF records at ONE
	/// name is a permerror (every check fails), while two records at different
	/// names are simply two domains.
	#[beet_core::test]
	fn one_spf_record_per_name() {
		let mut names = HashMap::<String, usize>::default();
		for (_, name, _) in record_values(&[staging(), news()])
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
		for (record_type, name) in records(&[staging(), news()]) {
			match record_type.as_str() {
				// the one apex-scoped name that is legitimately published, and
				// it is not a mail record: an atproto handle is namespaced under
				// the member's own label.
				"TXT" if name.starts_with("_atproto.") => continue,
				_ => {}
			}
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
		records(&[apex])
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
			]);
	}

	/// A block with no zone declares the identity and stays out of dns
	/// entirely, which is how a domain whose records somebody else holds still
	/// gets a verified sending identity.
	#[beet_core::test]
	fn no_zone_emits_no_records() {
		let block = MailDomainBlock::new("beetmash.com", "mail.beetmash.com");
		records(&[block.clone()]).xpect_eq(Vec::new());
		build_config(&[block])
			.to_json()
			.to_string()
			.xpect_contains("aws_sesv2_email_identity");
	}

	/// The DKIM records read the selectors off the identity's computed tokens
	/// rather than restating the ones a console showed, so a rotated key is a
	/// re-apply and not a hunt through the zone.
	#[beet_core::test]
	fn dkim_records_reference_the_identity_outputs() {
		let (_, _, content) = record_values(&[staging()])
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
		build_config(&[staging(), news()])
			.to_json()
			.to_string()
			.xpect_contains(
				"\"configuration_set_name\":\"stalwart-beetmash-com\"",
			)
			.xpect_contains("\"configuration_set_name\":\"news-beetmash-com\"");
	}

	/// The feedback host is regional and the region belongs to the stack, so a
	/// relocated stack moves it without the block naming a region at all.
	#[beet_core::test]
	fn mail_from_mx_follows_the_stack_region() {
		record_values(&[staging()])
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
		let values = record_values(&[news()]);
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

	/// Handles are an organisation-level name, so the block that names a handle
	/// domain publishes them and the ones that do not stay silent. Without this
	/// two mail domains would each publish the same `_atproto` records and race
	/// for the same terraform address.
	#[beet_core::test]
	fn handles_are_published_once_and_only_with_a_did() {
		records(&[staging(), news()])
			.into_iter()
			.filter(|(_, name)| name.starts_with("_atproto."))
			.collect::<Vec<_>>()
			.xpect_eq(vec![(
				"TXT".to_string(),
				// `info` was declared without a DID, so has no handle to publish
				"_atproto.pete.beetmash.com".to_string(),
			)]);
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
		records(&[staging()])
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
			let (stack, deployment, _dir) = ResolvedStack::default_local();
			let mut config = deployment.create_config(&stack);
			let mut world = World::new();
			block
				.apply_to_config(
					&world.spawn(()).as_readonly(),
					&stack,
					&deployment,
					&default(),
					&mut config,
				)
				.unwrap_err()
				.to_string()
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
}
