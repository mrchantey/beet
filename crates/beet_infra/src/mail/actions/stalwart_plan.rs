//! Everything the mail stack declares *inside* Stalwart's data store.
use crate::prelude::*;
use beet_core::prelude::*;
use serde_json::Value;
use serde_json::json;

/// The configuration a [`StalwartBlock`] and its [`MailDomainBlock`]s add up
/// to, as the JMAP objects `0.16` stores them.
///
/// `0.16` abolished the configuration file: the on-disk `config.json` describes
/// the four stores and nothing else, and listeners, TLS, routing, the spam
/// filter, domains and accounts all live in the data store as objects. So this
/// is the moral equivalent of the config file the old release had, built from
/// the same block declarations that emit the DNS records and the box, and
/// applied by [`StalwartProvision`].
///
/// Rendered without touching a server, which is what makes it testable: the
/// object shapes are asserted here against the pinned release's schema, and
/// only a live apply proves the server accepts them.
#[derive(Debug, Clone, PartialEq)]
pub struct StalwartPlan {
	/// The box's own name, ie its SMTP banner and its certificate subject.
	pub hostname: SmolStr,
	/// The address to register the ACME account with. Certificate expiry
	/// warnings go here, so it is a mailbox somebody reads.
	pub acme_contact: String,
	/// One listener per open port.
	pub listeners: Vec<Value>,
	/// The relay route every outbound message takes, ie SES.
	pub relay: Value,
	/// The domains served, in declaration order.
	pub domains: Vec<DomainPlan>,
}

/// One served domain and the accounts on it.
#[derive(Debug, Clone, PartialEq)]
pub struct DomainPlan {
	pub name: SmolStr,
	/// Hostnames the domain's automatic certificate must cover, ie every name a
	/// client opens TLS to.
	pub certificate_names: Vec<String>,
	/// The mailbox every otherwise unmatched address delivers to, as a
	/// localpart. Applied after the accounts exist, since it names one.
	pub catch_all: Option<SmolStr>,
	pub accounts: Vec<AccountPlan>,
	/// Where this domain's sovereign DKIM private key is parked, ie what
	/// `<EnsureDkimKey/>` minted and the selector record published the public
	/// half of.
	pub dkim_secret: SecretRef,
}

/// One mailbox, and every localpart that reaches it.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountPlan {
	/// The localpart, which in `0.16` is also the account name.
	pub name: SmolStr,
	/// Full management access. Exactly the accounts that administer the server.
	pub admin: bool,
	/// The localparts that deliver here, ie this mailbox's [`Alias`]es.
	pub aliases: Vec<SmolStr>,
	/// The [`SecretRef`] this account's generated password is parked at, so a
	/// probe (and a human) can authenticate as it later.
	pub secret: SecretRef,
}

/// The wire shape of the registry's `Map` type: one key per item, every value
/// literally `true`. A plain JSON array is rejected (`invalidPatch`), and so is
/// anything else, so every set-of-strings property in the schema (`contact`,
/// `subjectAlternativeNames`, `bind`) goes through here.
fn set_map<I, S>(items: I) -> Value
where
	I: IntoIterator<Item = S>,
	S: ToString,
{
	Value::Object(
		items
			.into_iter()
			.map(|item| (item.to_string(), Value::Bool(true)))
			.collect(),
	)
}

impl StalwartPlan {
	/// The ACME directory the certificates come from. Let's Encrypt, matching
	/// the `letsencrypt.org` rows the zone's `CAA` records already carry.
	pub const ACME_DIRECTORY: &'static str =
		"https://acme-v02.api.letsencrypt.org/directory";

	/// The route name the outbound strategy sends every remote message to. Not
	/// `mx`: an MTA relaying through SES never dials a recipient's MX itself,
	/// which is the entire point of not fighting an IP's reputation.
	pub const RELAY_ROUTE: &'static str = "ses";

	/// The route name Stalwart delivers local mail on, which is the one branch
	/// of the outbound strategy that must survive the relay override.
	pub const LOCAL_ROUTE: &'static str = "local";

	/// The listeners a freshly claimed server seeds that this stack does not
	/// declare, and which provision therefore removes.
	///
	/// Two are protocols nobody here speaks (`pop3s`, `sieve`), and the third
	/// is the plaintext management port the commissioning phase used. All three
	/// are already unreachable through the security group, so this is defence
	/// in depth rather than a fix: a listener bound on the box is a listener
	/// one wrong group rule away from the internet, and a management endpoint
	/// bound on loopback answers to anyone who reaches the box at all.
	///
	/// `http` retires LAST, after the certificate exists, because until then it
	/// is the only channel provision itself has.
	pub const RETIRED_LISTENERS: &'static [&'static str] =
		&["pop3s", "sieve", "http"];

	/// Read the plan off the blocks that declared it.
	///
	/// `admin_contact` is the address ACME registers and reports are addressed
	/// to; `ses` is the SMTP credential the relay route authenticates with,
	/// read from parameter store by the caller rather than by the plan, so a
	/// rendered plan can be asserted without a secret in it.
	///
	/// Only the domains whose records this stack serves are planned. An
	/// [`IdentityOnly`](MailRecords::IdentityOnly) domain is a cutover
	/// prepared ahead of its window: its identity must verify while the
	/// incumbent provider keeps the mail, and a local domain on the server
	/// would hijack every submission addressed to it away from the MX the
	/// world still resolves.
	pub fn new(
		mail_box: &StalwartBlock,
		domains: &[MailDomainBlock],
		stack: &ResolvedStack,
		admin_contact: &str,
		ses: &SesCredential,
	) -> Result<Self> {
		let hostname = mail_box.hostname().clone();
		let domains = domains
			.iter()
			.filter(|domain| domain.records().serves_mail())
			.enumerate()
			.map(|(index, domain)| {
				DomainPlan::new(domain, mail_box, index == 0)
			})
			.collect::<Result<Vec<_>>>()?;
		Ok(Self {
			acme_contact: admin_contact.to_string(),
			listeners: StalwartBlock::OPEN_PORTS
				.iter()
				.filter_map(|(port, service)| Self::listener(*port, service))
				.collect(),
			relay: Self::relay(stack, ses),
			hostname,
			domains,
		})
	}

	/// The ACME provider object, ie how every certificate is obtained.
	///
	/// `TlsAlpn01` and no port 80: the challenge is answered on the 443 the box
	/// already serves, so the security group needs no extra hole and the mail
	/// stack holds no dns credential.
	pub fn acme_provider(&self) -> Value {
		json!({
			"directory": Self::ACME_DIRECTORY,
			"challengeType": "TlsAlpn01",
			"contact": set_map([&self.acme_contact]),
		})
	}

	/// One listener, or none for a port that carries no listener of its own.
	///
	/// `22` is the box's sshd, which is a security-group opening rather than a
	/// Stalwart listener, so the one port list stays the single answer to "what
	/// is open" without claiming Stalwart serves it.
	fn listener(port: i64, service: &str) -> Option<Value> {
		// implicit TLS everywhere the protocol has a TLS port of its own; `25`
		// is STARTTLS by definition (a peer MTA dials it in the clear) and
		// `587` is the submission port clients still expect to negotiate on.
		let (protocol, implicit) = match service {
			"smtp" => ("smtp", false),
			"submission" => ("smtp", false),
			"submissions" => ("smtp", true),
			"imaps" => ("imap", true),
			"https" => ("http", true),
			_ => return None,
		};
		json!({
			"name": service,
			"protocol": protocol,
			"bind": set_map([format!("[::]:{port}")]),
			"useTls": true,
			"tlsImplicit": implicit,
		})
		.xmap(Some)
	}

	/// The relay route: every outbound message submitted to SES on 587 with
	/// STARTTLS, authenticating as the dedicated sending user.
	///
	/// `implicitTls: false` and port 587 rather than 465 because that is the
	/// endpoint SES documents for SMTP relay; the session is still TLS, just
	/// negotiated after the greeting.
	fn relay(stack: &ResolvedStack, ses: &SesCredential) -> Value {
		json!({
			"@type": "Relay",
			"name": Self::RELAY_ROUTE,
			"description": "Amazon SES relay",
			"address": StalwartBlock::ses_smtp_endpoint(stack),
			"port": 587,
			"protocol": "smtp",
			"implicitTls": false,
			"allowInvalidCerts": false,
			"authUsername": ses.username,
			"authSecret": { "@type": "Value", "secret": ses.password },
		})
	}

	/// The outbound strategy patch that puts [`RELAY_ROUTE`](Self::RELAY_ROUTE)
	/// in front of every remote delivery.
	///
	/// The local branch is restated rather than dropped: without it a message
	/// between two mailboxes on this server would be handed to SES and arrive
	/// back through the front door, which is a loop with a bill attached.
	pub fn outbound_strategy(&self) -> Value {
		json!({
			"route": {
				"else": format!("'{}'", Self::RELAY_ROUTE),
				// a `List` on the wire is an object keyed by index, never an
				// array, exactly like an account's aliases
				"match": { "0": {
					"if": "is_local_domain(rcpt_domain)",
					"then": format!("'{}'", Self::LOCAL_ROUTE),
				}},
			}
		})
	}

	/// The spam filter, on with auto-learning.
	///
	/// Learning from replies and from spam traps is what makes the classifier
	/// improve without anybody training it by hand, which for a server with a
	/// handful of mailboxes is the difference between a filter and a
	/// decoration. Scores are the shipped defaults: tagging at 5 and never
	/// rejecting outright, since a false positive that bounces is mail lost.
	pub fn spam_settings(&self) -> Value {
		json!({ "enable": true, "scoreSpam": 5.0, "scoreReject": 0.0, "scoreDiscard": 0.0 })
	}

	/// The classifier's auto-learn wiring, the other half of
	/// [`spam_settings`](Self::spam_settings).
	pub fn spam_classifier(&self) -> Value {
		json!({
			"learnHamFromReply": true,
			"learnHamFromCard": true,
			"learnSpamFromTraps": true,
			"learnSpamFromRblHits": 2,
		})
	}

	/// The system settings patch: the box's own name, which is what its SMTP
	/// greeting, its reports and its discovery documents all publish.
	pub fn system_settings(&self, default_domain_id: &str) -> Value {
		json!({
			"defaultHostname": self.hostname,
			"defaultDomainId": default_domain_id,
		})
	}
}

/// The SES SMTP credential the relay route authenticates with, read out of the
/// two parameters terraform parked it in.
///
/// A pair rather than two loose strings so a caller cannot hand the password in
/// as the username, which SES would report only as a delivery failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SesCredential {
	pub username: String,
	pub password: String,
}

impl DomainPlan {
	fn new(
		domain: &MailDomainBlock,
		mail_box: &StalwartBlock,
		is_first: bool,
	) -> Result<Self> {
		domain.validate()?;
		let name = domain.domain().clone();
		// every name a client opens TLS to, since ACME issues for the SANs and
		// nothing else: the autoconfig hosts that CNAME at the box, plus (on
		// the FIRST domain) the box's own hostname, which is the name an SMTP
		// peer and an IMAP client dial and which belongs to no mail domain at
		// all. The bare domain is deliberately ABSENT: a mail domain is MX and
		// TXT records with no address of its own, nothing dials TLS at it, and
		// a TLS-ALPN order naming it dies with "no valid A records", taking
		// every other name in the order down with it.
		let mut certificate_names = MailDomainBlock::AUTOCONFIG_LABELS
			.iter()
			.map(|label| format!("{label}.{name}"))
			.collect::<Vec<_>>();
		if is_first {
			certificate_names.push(mail_box.hostname().to_string());
		}
		let accounts = domain
			.mailboxes()
			.iter()
			.map(|mailbox| AccountPlan::new(mailbox, domain, mail_box.label()))
			.collect::<Vec<_>>();
		Ok(Self {
			certificate_names,
			catch_all: domain.catch_all().clone(),
			dkim_secret: domain.dkim_secret(),
			accounts,
			name,
		})
	}

	/// The domain object, minus the catch-all: that names a mailbox, so it is
	/// patched on once the accounts exist.
	///
	/// `dkimManagement: Manual` is deliberate. Outbound mail is signed by SES
	/// Easy DKIM, whose selectors are published as `CNAME`s by the domain
	/// block; a second, server-held selector is a later step, and generating
	/// one now would put a key in the data store that no record points at.
	pub fn object(&self, acme_provider_id: &str) -> Value {
		json!({
			"name": self.name,
			"isEnabled": true,
			"certificateManagement": {
				"@type": "Automatic",
				"acmeProviderId": acme_provider_id,
				"subjectAlternativeNames": set_map(&self.certificate_names),
			},
			"dkimManagement": { "@type": "Manual" },
		})
	}

	/// The sovereign DKIM signature object: this domain's own key, signing
	/// under [`MailDomainBlock::DKIM_SELECTOR`].
	///
	/// `active` from the moment it is created, because the record carrying its
	/// public half was published by the apply that ran before this: a signature
	/// staged as pending would sign nothing while a perfectly good record
	/// advertised it. The SES Easy DKIM signature keeps signing beside it —
	/// a verifier needs one of the two to pass, which is what makes the relay
	/// replaceable without a deliverability cliff.
	///
	/// `private_key` is present only on creation, exactly as an account's
	/// credential is: a key rotated under a published selector is a fortnight
	/// of mail signed by something no resolver can check.
	/// The headers the sovereign signature covers: exactly the ones that
	/// survive the relay. SES rewrites `Message-ID` and `Date` on every
	/// message it sends, and both sit in Stalwart's DEFAULT signed set, so a
	/// default-configured signature is broken by its own relay the moment the
	/// message leaves — while SES's own two signatures pass, because they are
	/// applied after the rewrite. Found by a Gmail round trip reading
	/// `dkim=fail header.s=stalwart` beside two passes. Headers absent from a
	/// message are signed as absent (RFC 6376 explicitly allows this), which
	/// also blocks their later addition.
	pub const SIGNED_HEADERS: &'static [&'static str] = &[
		"From",
		"To",
		"Cc",
		"Reply-To",
		"Subject",
		"MIME-Version",
		"Content-Type",
		"Content-Transfer-Encoding",
	];

	pub fn dkim_object(
		&self,
		domain_id: &str,
		private_key: Option<&str>,
	) -> Value {
		let mut object = json!({
			"@type": "Dkim1RsaSha256",
			"selector": MailDomainBlock::DKIM_SELECTOR,
			"domainId": domain_id,
			"stage": "active",
			"headers": set_map(Self::SIGNED_HEADERS),
		});
		if let Some(private_key) = private_key {
			object["privateKey"] =
				json!({ "@type": "Text", "secret": private_key });
		}
		object
	}

	/// The catch-all patch, as the full address the mailbox answers at.
	pub fn catch_all_patch(&self) -> Option<Value> {
		self.catch_all
			.as_ref()
			.map(|localpart| json!({ "catchAllAddress": format!("{localpart}@{}", self.name) }))
	}
}

impl AccountPlan {
	/// The parameter one mailbox's generated password is parked at, ie
	/// `/beetmash/prod/mail-account-pete-at-stalwart-beetmash-com`.
	///
	/// Composed HERE and nowhere else because five things read it and none of
	/// them can see each other: the provision that mints it, the probe that
	/// signs in with it, the restore drill that proves a restored store still
	/// answers to it, the credential listing a human reads, and the account
	/// object it belongs to. A sixth hand-written copy is the bug this exists
	/// to prevent, and its failure mode is an authentication error three steps
	/// away from the typo.
	pub fn secret_ref(
		box_label: &str,
		localpart: &str,
		domain_slug: &str,
	) -> SecretRef {
		SecretRef::new(format!(
			"{box_label}-account-{localpart}-at-{domain_slug}"
		))
	}

	fn new(
		mailbox: &Mailbox,
		domain: &MailDomainBlock,
		box_label: &str,
	) -> Self {
		let name = mailbox.localpart().clone();
		// every alias on the domain that targets this mailbox. Read from the
		// domain rather than restated on the mailbox, because an alias is a
		// property of the pair and the domain block already validated that the
		// target exists.
		let aliases = domain
			.aliases()
			.iter()
			.filter(|alias| alias.target() == &name)
			.map(|alias| alias.localpart().clone())
			.collect();
		Self {
			secret: Self::secret_ref(box_label, &name, &domain.slug()),
			admin: mailbox.admin(),
			aliases,
			name,
		}
	}

	/// The address this account receives at.
	pub fn address(&self, domain: &str) -> String {
		format!("{}@{domain}", self.name)
	}

	/// The account object.
	///
	/// `password` is the generated value parked at [`secret`](Self::secret),
	/// and is present only on creation: an account that already exists keeps
	/// the credential it has, since rotating it on every deploy would lock out
	/// every client configured against it.
	pub fn object(&self, domain_id: &str, password: Option<&str>) -> Value {
		let mut object = json!({
			"@type": "User",
			"name": self.name,
			"domainId": domain_id,
			"roles": { "@type": if self.admin { "Admin" } else { "User" } },
			"aliases": self
				.aliases
				.iter()
				.enumerate()
				.map(|(index, alias)| (
					index.to_string(),
					json!({ "name": alias, "domainId": domain_id, "enabled": true }),
				))
				.collect::<serde_json::Map<_, _>>(),
		});
		if let Some(password) = password {
			object["credentials"] = json!({
				"0": { "@type": "Password", "secret": password }
			});
		}
		object
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn stack() -> ResolvedStack {
		Stack::new("beetmash")
			.with_stage("prod")
			.with_region(crate::bindings::aws::region::AP_SOUTHEAST_2)
			.resolve(&PackageConfig::default())
	}

	fn mail_box() -> StalwartBlock {
		StalwartBlock::new("mail", "mail.beetmash.com")
			.with_blob_bucket("mail-blobs")
			.with_ssh_public_key("ssh-ed25519 AAAAC3TESTKEY pete")
	}

	/// The staging domain as the plan declares it, with the mailbox set from
	/// decision 13 and the role aliases every served domain publishes.
	fn staging() -> MailDomainBlock {
		MailDomainBlock::new("stalwart.beetmash.com", "mail.beetmash.com")
			.with_admin_member(Member::new("pete"))
			.with_mailbox(Mailbox::new("info"))
			.with_mailbox(Mailbox::new("publications"))
			.with_mailbox(Mailbox::new("probe"))
			.with_role_aliases("pete", "info")
	}

	fn news() -> MailDomainBlock {
		MailDomainBlock::new("news.beetmash.com", "mail.beetmash.com")
			.with_mailbox(Mailbox::new("publications"))
			.with_catch_all("publications")
	}

	fn plan() -> StalwartPlan {
		StalwartPlan::new(
			&mail_box(),
			&[staging(), news()],
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&SesCredential {
				username: "AKIATEST".into(),
				password: "smtp-derived-password".into(),
			},
		)
		.unwrap()
	}

	/// Every open port that carries a Stalwart protocol gets a listener, and
	/// port 22 does not: the box's sshd is a security-group opening, not
	/// something Stalwart serves. Mail ports never bind at all until these
	/// exist, so a missing one is a silently dead protocol.
	///
	/// `bind` is the registry's `Map` wire shape (`{addr: true}`), never an
	/// array: an array is an `invalidPatch` the server rejects at apply.
	#[beet_core::test]
	fn a_listener_per_served_port() {
		let plan = plan();
		let ports = plan
			.listeners
			.iter()
			.map(|listener| {
				let bind = listener["bind"].as_object().unwrap();
				bind.values()
					.all(|value| value.as_bool() == Some(true))
					.xpect_true();
				bind.keys().next().unwrap().to_string()
			})
			.collect::<Vec<_>>();
		ports.len().xpect_eq(5);
		ports.contains(&"[::]:25".to_string()).xpect_true();
		ports.contains(&"[::]:443".to_string()).xpect_true();
		ports.contains(&"[::]:465".to_string()).xpect_true();
		ports.contains(&"[::]:587".to_string()).xpect_true();
		ports.contains(&"[::]:993".to_string()).xpect_true();
		ports.contains(&"[::]:22".to_string()).xpect_false();
	}

	/// `0.16` stopped adding plaintext listeners by default and only advertises
	/// implicit-TLS ports, so which of ours negotiate and which are implicit is
	/// a decision rather than an inherited default: 25 and 587 negotiate, 465
	/// 993 and 443 are implicit.
	#[beet_core::test]
	fn implicit_tls_where_the_port_says_so() {
		let implicit = |name: &str| {
			plan()
				.listeners
				.iter()
				.find(|listener| listener["name"] == name)
				.unwrap()["tlsImplicit"]
				.as_bool()
				.unwrap()
		};
		implicit("smtp").xpect_false();
		implicit("submission").xpect_false();
		implicit("submissions").xpect_true();
		implicit("imaps").xpect_true();
		implicit("https").xpect_true();
	}

	/// The relay is the whole outbound design: SES on 587, authenticating with
	/// the credential terraform derived, and never dialling a recipient's MX.
	#[beet_core::test]
	fn outbound_goes_to_ses_and_local_mail_does_not() {
		let plan = plan();
		plan.relay["@type"].as_str().unwrap().xpect_eq("Relay");
		plan.relay["address"]
			.as_str()
			.unwrap()
			.xpect_eq("email-smtp.ap-southeast-2.amazonaws.com");
		plan.relay["port"].as_i64().unwrap().xpect_eq(587);
		plan.relay["implicitTls"].as_bool().unwrap().xpect_false();
		plan.relay["authUsername"]
			.as_str()
			.unwrap()
			.xpect_eq("AKIATEST");

		let strategy = plan.outbound_strategy();
		strategy["route"]["else"]
			.as_str()
			.unwrap()
			.xpect_eq("'ses'");
		// without the local branch a message between two mailboxes on this
		// server would go out to SES and come back in through the front door.
		// (a `List` on the wire is keyed by index, never an array)
		strategy["route"]["match"]["0"]["then"]
			.as_str()
			.unwrap()
			.xpect_eq("'local'");
	}

	/// The certificate must cover every name a client opens TLS to, and the
	/// box's own hostname belongs to no mail domain at all: it rides the first
	/// domain's SANs, which is the only place it can.
	///
	/// REGRESSION: the bare domain was a SAN, and a mail domain is MX and TXT
	/// records with no A record of its own, so Let's Encrypt answered "no
	/// valid A records found" and the whole order — every valid name included
	/// — failed with it. A name belongs here exactly when something resolves
	/// it to the box.
	#[beet_core::test]
	fn the_certificate_covers_the_box_and_the_autoconfig_hosts() {
		let plan = plan();
		let first = &plan.domains[0].certificate_names;
		first
			.contains(&"stalwart.beetmash.com".to_string())
			.xpect_false();
		first
			.contains(&"autoconfig.stalwart.beetmash.com".to_string())
			.xpect_true();
		first
			.contains(&"autodiscover.stalwart.beetmash.com".to_string())
			.xpect_true();
		first
			.contains(&"mail.beetmash.com".to_string())
			.xpect_true();
		// and exactly once across the stack, since two domains requesting the
		// same name would each renew a certificate for it.
		plan.domains[1]
			.certificate_names
			.contains(&"mail.beetmash.com".to_string())
			.xpect_false();
	}

	/// An `IdentityOnly` domain is a cutover prepared ahead of its window: the
	/// SES identity verifies and its selectors publish while the incumbent
	/// provider keeps serving the mail, so the server must not hold it.
	///
	/// REGRESSION for the two ways holding it goes wrong. A local domain
	/// hijacks every submission addressed to it, so a soak-period user writing
	/// to the apex would land in an empty local mailbox instead of at the
	/// incumbent the MX still names. And its autoconfig host resolves nowhere
	/// until the delivery records publish, which is the same "no valid A
	/// records" failure the bare-domain SAN produced, killing the ACME order
	/// for every domain beside it.
	#[beet_core::test]
	fn a_cutover_staged_domain_is_not_provisioned() {
		let apex = MailDomainBlock::new("beetmash.com", "mail.beetmash.com")
			.with_records(MailRecords::IdentityOnly);
		let plan = StalwartPlan::new(
			&mail_box(),
			&[staging(), news(), apex],
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&SesCredential {
				username: "AKIATEST".into(),
				password: "smtp-derived-password".into(),
			},
		)
		.unwrap();
		plan.domains
			.iter()
			.any(|domain| domain.name == "beetmash.com")
			.xpect_false();
		// and the box hostname still rides the first SERVED domain's
		// certificate, so the filter cannot orphan it.
		plan.domains[0]
			.certificate_names
			.contains(&"mail.beetmash.com".to_string())
			.xpect_true();
	}

	/// An alias is a property of the domain, and lands on the account it
	/// targets: `postmaster@` reaches pete, `dmarc@` reaches info, and nothing
	/// reaches an account that did not declare it.
	#[beet_core::test]
	fn aliases_land_on_the_mailbox_they_target() {
		let plan = plan();
		let staging = &plan.domains[0];
		let account = |name: &str| {
			staging
				.accounts
				.iter()
				.find(|account| account.name == name)
				.unwrap()
		};
		account("pete")
			.aliases
			.iter()
			.map(SmolStr::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(
				Alias::OPERATOR_ROLES
					.iter()
					.map(ToString::to_string)
					.collect::<Vec<_>>(),
			);
		account("info")
			.aliases
			.iter()
			.map(SmolStr::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec!["dmarc".to_string(), "tlsrpt".to_string()]);
		account("probe").aliases.is_empty().xpect_true();
	}

	/// Exactly the accounts that administer the server get the admin role. The
	/// probe mailbox authenticates to send and read its own mail and must not
	/// be able to reconfigure the server it is probing.
	#[beet_core::test]
	fn only_declared_admins_get_the_admin_role() {
		let plan = plan();
		let staging = &plan.domains[0];
		let role = |name: &str| {
			staging
				.accounts
				.iter()
				.find(|account| account.name == name)
				.unwrap()
				.object("d1", None)["roles"]["@type"]
				.as_str()
				.unwrap()
				.to_string()
		};
		role("pete").xpect_eq("Admin");
		role("probe").xpect_eq("User");
		role("info").xpect_eq("User");
	}

	/// A member declaration carries admin rights through to the account, which
	/// is the one place the server's own administrator is decided.
	#[beet_core::test]
	fn a_member_mailbox_can_be_declared_admin() {
		let domain =
			MailDomainBlock::new("stalwart.beetmash.com", "mail.beetmash.com")
				.with_mailbox(
					Mailbox::new("pete").with_admin(true).with_member("pete"),
				);
		StalwartPlan::new(
			&mail_box(),
			&[domain],
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&SesCredential {
				username: "AKIATEST".into(),
				password: "x".into(),
			},
		)
		.unwrap()
		.domains[0]
			.accounts[0]
			.admin
			.xpect_true();
	}

	/// The credential rides the CREATE only. An account that already exists
	/// keeps the password parameter store says it has, or every deploy would
	/// lock out every configured mail client.
	#[beet_core::test]
	fn the_password_is_written_once() {
		let plan = plan();
		let account = &plan.domains[0].accounts[0];
		account.object("d1", Some("secret"))["credentials"]["0"]["secret"]
			.as_str()
			.unwrap()
			.xpect_eq("secret");
		account
			.object("d1", None)
			.get("credentials")
			.is_none()
			.xpect_true();
	}

	/// Each mailbox's password parks under the stack's own secret prefix, so
	/// the instance role's one statement already covers it and a probe knows
	/// where to look without being told.
	#[beet_core::test]
	fn account_secrets_park_under_the_stack_prefix() {
		let plan = plan();
		plan.domains[0].accounts[3].secret.name(&stack()).xpect_eq(
			"/beetmash/prod/mail-account-probe-at-stalwart-beetmash-com",
		);
	}

	/// The catch-all is the publication domain's whole point (a reply to a
	/// newsletter is welcome and unpredictable) and the person's domain's
	/// opposite (a catch-all is a spam trap that accepts).
	#[beet_core::test]
	fn only_a_publication_domain_catches_all() {
		let plan = plan();
		plan.domains[0].catch_all_patch().is_none().xpect_true();
		plan.domains[1].catch_all_patch().unwrap()["catchAllAddress"]
			.as_str()
			.unwrap()
			.xpect_eq("publications@news.beetmash.com");
	}

	/// ACME answers on the 443 the box already serves, so no port 80 opens and
	/// the mail stack holds no dns credential. The directory is Let's Encrypt,
	/// which is what the zone's existing `CAA` rows authorise.
	#[beet_core::test]
	fn acme_answers_on_the_port_already_open() {
		let provider = plan().acme_provider();
		provider["challengeType"]
			.as_str()
			.unwrap()
			.xpect_eq("TlsAlpn01");
		provider["directory"]
			.as_str()
			.unwrap()
			.xpect_contains("letsencrypt.org");
		// the `Map` wire shape: the address IS the key
		provider["contact"]["postmaster@stalwart.beetmash.com"]
			.as_bool()
			.unwrap()
			.xpect_true();
	}

	/// Outbound mail is signed by SES Easy DKIM, whose selectors the domain
	/// block publishes. A server-held selector generated here would put a key in
	/// the data store that no record points at.
	#[beet_core::test]
	fn dkim_stays_with_ses_until_a_selector_is_published() {
		plan().domains[0].object("acme1")["dkimManagement"]["@type"]
			.as_str()
			.unwrap()
			.xpect_eq("Manual");
	}

	/// The plan carries the secret only where the relay route needs it, and a
	/// rendered plan is otherwise safe to log: everything else is names.
	#[beet_core::test]
	fn the_ses_password_appears_only_on_the_relay_route() {
		let plan = plan();
		let rendered =
			serde_json::to_string(&plan.domains[0].object("acme1")).unwrap();
		rendered.contains("smtp-derived-password").xpect_false();
		plan.relay["authSecret"]["secret"]
			.as_str()
			.unwrap()
			.xpect_eq("smtp-derived-password");
	}

	/// The signature the domain publishes a selector for: this stack's own key,
	/// active immediately because the record carrying its public half was
	/// published by the apply that ran before the provision.
	#[beet_core::test]
	fn the_sovereign_signature_is_active_on_creation() {
		let plan = DomainPlan::new(&staging(), &mail_box(), true).unwrap();
		let object =
			plan.dkim_object("d1", Some("-----BEGIN PRIVATE KEY-----"));
		object["@type"].as_str().unwrap().xpect_eq("Dkim1RsaSha256");
		object["selector"]
			.as_str()
			.unwrap()
			.xpect_eq(MailDomainBlock::DKIM_SELECTOR);
		object["stage"].as_str().unwrap().xpect_eq("active");
		object["privateKey"]["@type"]
			.as_str()
			.unwrap()
			.xpect_eq("Text");
	}

	/// The signed header set names no header the SES relay rewrites.
	///
	/// REGRESSION: the object declared no `headers`, so the server signed its
	/// default set, which includes `Message-ID` and `Date` — the two headers
	/// SES replaces on every message it sends. Every real message therefore
	/// left the box with a sovereign signature its own relay had already
	/// broken (`dkim=fail header.s=stalwart` at Gmail, beside two SES
	/// passes), while the probe's inbound leg — signed by SES, not by us —
	/// stayed green through it.
	#[beet_core::test]
	fn the_signature_survives_the_relay() {
		let object = DomainPlan::new(&staging(), &mail_box(), true)
			.unwrap()
			.dkim_object("d1", None);
		let headers = object["headers"].as_object().unwrap();
		headers.contains_key("Message-ID").xpect_false();
		headers.contains_key("Date").xpect_false();
		headers["From"].as_bool().unwrap().xpect_true();
		headers["Subject"].as_bool().unwrap().xpect_true();
	}

	/// The key is absent from the form convergence MATCHES on, exactly as an
	/// account's password is: a key rotated under a published selector signs
	/// mail no verifier can check until dns catches up.
	#[beet_core::test]
	fn the_signing_key_is_written_once() {
		DomainPlan::new(&staging(), &mail_box(), true)
			.unwrap()
			.dkim_object("d1", None)["privateKey"]
			.is_null()
			.xpect_true();
	}
}
