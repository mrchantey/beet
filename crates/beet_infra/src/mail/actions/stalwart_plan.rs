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
	/// The relay routes outbound mail takes, one per distinct credential: a
	/// shared `ses` route when any domain relays through SES, plus one
	/// `comail-<slug>` per enrolled domain.
	pub relays: Vec<Value>,
	/// Which route each RELAYED domain's mail leaves on, as
	/// `(sender domain, route name)` in declaration order. A domain delivering
	/// directly is absent, which is what makes it fall through to `mx`.
	pub routing: Vec<(SmolStr, String)>,
	/// The route the outbound strategy falls through to: `mx` when any domain
	/// delivers directly, else the route the most domains relay through.
	pub fallback: String,
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

	/// The route name Stalwart delivers local mail on, which is the one branch
	/// of the outbound strategy that must survive any relay override.
	pub const LOCAL_ROUTE: &'static str = "local";

	/// Stalwart's built-in route, ie dial the recipient's own `MX`. What an
	/// unconfigured server does, and what a domain declaring no relay wants:
	/// the box delivers its own mail on its own address.
	pub const MX_ROUTE: &'static str = "mx";

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
	/// to; `relays` is each domain's resolved provider and `credentials` holds
	/// one SMTP credential per ROUTE, read from parameter store by the caller
	/// rather than by the plan, so a rendered plan can be asserted without a
	/// secret in it.
	///
	/// Routing is per SENDER domain rather than one blanket override, because a
	/// comail credential pins the single domain its session may send from
	/// (`internal/relay/smtp.go:391`): two enrolled domains are two routes with
	/// two credentials, and handing either's mail to the other's route is a
	/// `550 Sender domain must be ...`.
	///
	/// Only the domains whose records this stack serves are planned. An
	/// [`IdentityOnly`](MailRecords::IdentityOnly) domain is a cutover
	/// prepared ahead of its window: its identity must verify while the
	/// incumbent provider keeps the mail, and a local domain on the server
	/// would hijack every submission addressed to it away from the MX the
	/// world still resolves.
	///
	/// DECLARATION ORDER IS LOAD-BEARING, and quietly so: the FIRST domain
	/// left after that filter is the box's primary, which decides two things
	/// no reader of the entry would guess from the order of its tags. It
	/// becomes `defaultDomainId` in
	/// [`system_settings`](Self::system_settings), which is what
	/// `system('domain')` resolves to, which is the domain every outbound
	/// report is addressed FROM (`noreply-dmarc@<primary>`). And it is the
	/// only domain whose certificate order carries the BOX's own hostname
	/// (see `DomainPlan::new`), since that name belongs to no mail domain
	/// and has to ride on one.
	///
	/// So reordering the tags in an entry, or deleting the first one, moves
	/// the server's identity and re-orders an ACME request. That is why
	/// `infra/mail.bsx` declares the apex directly after the staging domain it
	/// replaces rather than last: when phase 12 deletes the staging block, the
	/// primary falls through to the apex instead of to the newsletter domain.
	pub fn new(
		mail_box: &StalwartBlock,
		domains: &[MailDomainBlock],
		relays: &RelayModes,
		stack: &ResolvedStack,
		admin_contact: &str,
		credentials: &HashMap<String, RelayCredential>,
	) -> Result<Self> {
		let hostname = mail_box.hostname().clone();
		let served = domains
			.iter()
			.filter(|domain| domain.records().serves_mail())
			.collect::<Vec<_>>();
		let (routes, routing) =
			Self::relay_routes(&served, relays, stack, credentials)?;
		// a served domain with no route is a domain delivering directly, which
		// is read off the routing rather than off `relays`: a stack renders
		// from the domains it declared, and a mode map missing one of them
		// would otherwise silently put its mail behind somebody else's relay.
		let any_direct = routing.len() < served.len();
		let domains = served
			.iter()
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
			fallback: Self::fallback(&routing, any_direct),
			relays: routes,
			routing,
			hostname,
			domains,
		})
	}

	/// The relay routes the served domains add up to, and which route each
	/// relayed domain's mail leaves on.
	///
	/// One shared [`SES_ROUTE`](SesRelay::ROUTE) however many domains relay
	/// through SES, and one route per COMAIL domain however few. Deliberately
	/// asymmetric, because the credentials are: SES's one IAM user sends for
	/// every identity in the account, while a comail key is issued per enrolled
	/// domain and pins it.
	fn relay_routes(
		served: &[&MailDomainBlock],
		relays: &RelayModes,
		stack: &ResolvedStack,
		credentials: &HashMap<String, RelayCredential>,
	) -> Result<(Vec<Value>, Vec<(SmolStr, String)>)> {
		let mut routes = Vec::new();
		let mut routing = Vec::new();
		let mut seen = HashSet::<String>::default();
		for domain in served {
			let relay = relays.get(domain.domain());
			let Some(name) = relay.route_name(&domain.slug()) else {
				continue;
			};
			routing.push((domain.domain().clone(), name.clone()));
			if !seen.insert(name.clone()) {
				continue;
			}
			let credential = credentials.get(&name).ok_or_else(|| {
				bevyhow!(
					"no credential for the '{name}' relay route, which \
					'{}' sends on",
					domain.domain()
				)
			})?;
			routes.push(match relay {
				RelayMode::Ses(_) => Self::ses_route(stack, credential),
				RelayMode::Comail(comail) => {
					Self::comail_route(&name, comail, credential)
				}
				RelayMode::None => continue,
			});
		}
		Ok((routes, routing))
	}

	/// Where a message whose sender domain matched no route goes: `mx` if any
	/// served domain delivers directly, else whichever route the most domains
	/// use.
	///
	/// The fallback is only ever reached by mail this stack's own domains did
	/// not send (a DMARC report addressed from the box's primary, a DSN), so
	/// the rule is deterministic rather than clever: honour a stack that has
	/// declared direct delivery anywhere, and otherwise stay behind the relay
	/// the stack mostly uses.
	fn fallback(routing: &[(SmolStr, String)], any_direct: bool) -> String {
		if any_direct || routing.is_empty() {
			return Self::MX_ROUTE.to_string();
		}
		let mut counts = HashMap::<&String, usize>::default();
		for (_, route) in routing {
			*counts.entry(route).or_default() += 1;
		}
		counts
			.into_iter()
			// count first, then name, so a tie resolves the same way every run
			.max_by(|left, right| {
				left.1.cmp(&right.1).then_with(|| right.0.cmp(left.0))
			})
			.map(|(route, _)| route.clone())
			.unwrap_or_else(|| Self::MX_ROUTE.to_string())
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

	/// The SES relay route: every outbound message submitted to SES on 587 with
	/// STARTTLS, authenticating as the dedicated sending user.
	///
	/// `implicitTls: false` and port 587 rather than 465 because that is the
	/// endpoint SES documents for SMTP relay; the session is still TLS, just
	/// negotiated after the greeting.
	fn ses_route(stack: &ResolvedStack, credential: &RelayCredential) -> Value {
		Self::route(
			SesRelay::ROUTE,
			"Amazon SES relay",
			&SesRelay::smtp_endpoint(stack),
			587,
			credential,
		)
	}

	/// One enrolled domain's comail route, on the same 587 STARTTLS shape.
	///
	/// There is no implicit-TLS port to prefer here: the relay serves 587 and
	/// only 587 (`cmd/relay/submission.go`), so the session negotiates or it
	/// does not open.
	fn comail_route(
		name: &str,
		comail: &ComailRelay,
		credential: &RelayCredential,
	) -> Value {
		Self::route(
			name,
			&format!("comail relay via {}", comail.host()),
			comail.host(),
			ComailRelay::SMTP_PORT,
			credential,
		)
	}

	/// One `Relay` route object.
	fn route(
		name: &str,
		description: &str,
		address: &str,
		port: i64,
		credential: &RelayCredential,
	) -> Value {
		json!({
			"@type": "Relay",
			"name": name,
			"description": description,
			"address": address,
			"port": port,
			"protocol": "smtp",
			"implicitTls": false,
			"allowInvalidCerts": false,
			"authUsername": credential.username,
			"authSecret": { "@type": "Value", "secret": credential.password },
		})
	}

	/// The outbound strategy patch: local mail first, then each relayed domain
	/// matched by its SENDER domain, then everything else on
	/// [`fallback`](Self::fallback).
	///
	/// The local branch is restated rather than dropped, and stays FIRST:
	/// without it a message between two mailboxes on this server would be
	/// handed to a relay and arrive back through the front door, which is a
	/// loop with a bill attached.
	///
	/// Sender domain rather than one blanket override, because that is the only
	/// key that can tell two comail credentials apart: the recipient says
	/// nothing about which of our domains is sending.
	pub fn outbound_strategy(&self) -> Value {
		// a `List` on the wire is an object keyed by index, never an array,
		// exactly like an account's aliases
		let mut branches = serde_json::Map::new();
		branches.insert(
			"0".to_string(),
			json!({
				"if": "is_local_domain(rcpt_domain)",
				"then": format!("'{}'", Self::LOCAL_ROUTE),
			}),
		);
		for (index, (domain, route)) in self.routing.iter().enumerate() {
			branches.insert(
				(index + 1).to_string(),
				json!({
					"if": format!("sender_domain == '{domain}'"),
					"then": format!("'{route}'"),
				}),
			);
		}
		json!({
			"route": {
				"else": format!("'{}'", self.fallback),
				"match": Value::Object(branches),
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

	/// Outbound authentication reporting: the aggregate half on, the
	/// per-message failure half off.
	///
	/// A server that checks DMARC is also a REPORTER, which is a sending
	/// obligation nothing here declared and this box acquired the moment it
	/// started accepting mail. The two halves are not the same trade.
	/// Aggregate (`rua`) reports are one message per reporting domain per day
	/// carrying counts, which is how the ecosystem works and how the `rua=`
	/// rows [`MailDomainBlock`] publishes get answered by everyone else.
	/// Failure (`ruf`) reports are one message per spoofed message, they carry
	/// the headers of mail addressed to OUR users out to whatever address a
	/// third party published, and almost nothing consumes them.
	///
	/// Found 2026-09-02 from two SES bounce notifications: spoofed
	/// `investing.com` mail arrived here, this server reported it to the
	/// `dmarc@investing.com` their record names for both `rua=` and `ruf=`,
	/// and Google bounced both reports. Reputation was never at risk, since
	/// transient bounces do not count toward the SES bounce rate and ours
	/// stayed at `0.0` through both days. But it is our SES quota and our
	/// bounce notifications spent on a third party's telemetry, and the
	/// failure half is the half whose volume tracks how much spam we
	/// receive.
	///
	/// Verified against the pinned tag rather than inferred, per decision 18.
	/// `daily` and `disable` are `ExpressionConstant` words, so they are
	/// UNQUOTED where a string literal in the same position would be quoted
	/// (`outbound_strategy`'s route names, which are strings, carry their
	/// quotes). The aggregate frequency maps `disable` to
	/// `AggregateFrequency::Never`; every failure frequency is instead a
	/// `[count, period]` rate whose conversion rejects anything that is not a
	/// two-element array of POSITIVE numbers, and the send paths read that
	/// rejection (`eval_if` turns the error into `None`) as "off". So one word
	/// turns both off, for two different reasons.
	pub fn dmarc_report_settings(&self) -> Value {
		json!({
			"aggregateSendFrequency": { "else": "daily" },
			"failureSendFrequency": { "else": "disable" },
		})
	}

	/// The DKIM and SPF authentication failure reports, off for the reason the
	/// DMARC failure half is off and by the same rate-rejection mechanism.
	///
	/// One shape for both singletons because both objects name the property
	/// `sendFrequency`, and both ship the same `[1, 1d]` default: a per-message
	/// report to whatever address a remote domain asked for. These are the
	/// pre-DMARC reporting mechanisms and they are quieter than `ruf` only
	/// because fewer domains still request them, which is not a reason to
	/// leave them on.
	///
	/// TLS-RPT is deliberately NOT here: `x:TlsReportSettings` is aggregate
	/// (`daily`), it is the other half of the reports the MTA-STS enforce
	/// flips are judged against, and it stays exactly as it is.
	pub fn auth_failure_report_settings(&self) -> Value {
		json!({ "sendFrequency": { "else": "disable" } })
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

/// The SMTP credential one relay route authenticates with, read out of the
/// parameters it was parked in.
///
/// A pair rather than two loose strings so a caller cannot hand the password in
/// as the username, which either provider would report only as a delivery
/// failure. The two arms park it differently and mean different things by it:
/// SES's pair is an IAM access key id and its derived SMTP password, written by
/// terraform; comail's is the enrolled DID and its `atmos_…` api key, written
/// by the operator from the enrolment response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayCredential {
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
	/// `dkim=fail header.s=stalwart` beside two passes.
	///
	/// The ADDRESS headers are out for the same reason, found at the apex
	/// cutover: SES re-parses and re-emits `To`, `Cc` and `Reply-To`, so a
	/// bare angle form (`To: <a@b.c>`, the shape every script and agent
	/// composes) is rewritten to `To: a@b.c` in transit and a signature
	/// covering it dies, while the display-name form happens to survive —
	/// which is why a round trip from a mail client passes and one from curl
	/// does not. `From` stays because RFC 6376 requires it signed; senders
	/// should compose it in display-name form, which the relay preserves.
	/// Headers absent from a message are signed as absent (RFC 6376
	/// explicitly allows this), which also blocks their later addition.
	pub const SIGNED_HEADERS: &'static [&'static str] = &[
		"From",
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

	/// Every domain relayed through `mode`, ie the arrangement a stack that
	/// declares one relay above its domains resolves to.
	fn all(domains: &[MailDomainBlock], mode: RelayMode) -> RelayModes {
		let mut relays = RelayModes::default();
		for domain in domains {
			relays.insert(domain.domain().clone(), mode.clone());
		}
		relays
	}

	/// One credential per route name, with the password naming the route so an
	/// assertion can tell two comail credentials apart.
	fn credentials(names: &[&str]) -> HashMap<String, RelayCredential> {
		names
			.iter()
			.map(|name| {
				(name.to_string(), RelayCredential {
					username: format!("user-{name}"),
					password: format!("password-{name}"),
				})
			})
			.collect()
	}

	/// The all-SES plan, ie exactly what this stack rendered before a domain
	/// could relay through anything else.
	fn plan() -> StalwartPlan {
		let domains = [staging(), news()];
		StalwartPlan::new(
			&mail_box(),
			&domains,
			&all(&domains, RelayMode::Ses(SesRelay::default())),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&HashMap::from_iter([(
				SesRelay::ROUTE.to_string(),
				RelayCredential {
					username: "AKIATEST".into(),
					password: "smtp-derived-password".into(),
				},
			)]),
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

	/// The relay is the whole outbound design when SES carries it: one route on
	/// 587, authenticating with the credential terraform derived, and never
	/// dialling a recipient's MX.
	#[beet_core::test]
	fn outbound_goes_to_ses_and_local_mail_does_not() {
		let plan = plan();
		plan.relays.len().xpect_eq(1);
		let relay = &plan.relays[0];
		relay["@type"].as_str().unwrap().xpect_eq("Relay");
		relay["name"].as_str().unwrap().xpect_eq("ses");
		relay["address"]
			.as_str()
			.unwrap()
			.xpect_eq("email-smtp.ap-southeast-2.amazonaws.com");
		relay["port"].as_i64().unwrap().xpect_eq(587);
		relay["implicitTls"].as_bool().unwrap().xpect_false();
		relay["authUsername"].as_str().unwrap().xpect_eq("AKIATEST");

		let strategy = plan.outbound_strategy();
		// every served domain is matched by name, and anything else falls
		// through to the only relay the stack has
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
		strategy["route"]["match"]["1"]["if"]
			.as_str()
			.unwrap()
			.xpect_eq("sender_domain == 'stalwart.beetmash.com'");
		strategy["route"]["match"]["1"]["then"]
			.as_str()
			.unwrap()
			.xpect_eq("'ses'");
	}

	/// Two comail domains are TWO routes with two credentials, never one shared
	/// route: the submission gate rejects an envelope sender whose domain is
	/// not the one the key enrolled
	/// (`internal/relay/smtp.go:391-403`, `550 Sender domain must be ...`), so
	/// a shared route would deliver one domain's mail and 550 the other's.
	#[beet_core::test]
	fn each_comail_domain_gets_its_own_route_and_credential() {
		let domains = [staging(), news()];
		let plan = StalwartPlan::new(
			&mail_box(),
			&domains,
			&all(&domains, RelayMode::Comail(ComailRelay::default())),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&credentials(&[
				"comail-stalwart-beetmash-com",
				"comail-news-beetmash-com",
			]),
		)
		.unwrap();
		plan.relays
			.iter()
			.map(|relay| {
				(
					relay["name"].as_str().unwrap().to_string(),
					relay["authUsername"].as_str().unwrap().to_string(),
					relay["address"].as_str().unwrap().to_string(),
					relay["port"].as_i64().unwrap(),
				)
			})
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				(
					"comail-stalwart-beetmash-com".to_string(),
					"user-comail-stalwart-beetmash-com".to_string(),
					"smtp.atmos.email".to_string(),
					587,
				),
				(
					"comail-news-beetmash-com".to_string(),
					"user-comail-news-beetmash-com".to_string(),
					"smtp.atmos.email".to_string(),
					587,
				),
			]);
		let strategy = plan.outbound_strategy();
		strategy["route"]["match"]["1"]["then"]
			.as_str()
			.unwrap()
			.xpect_eq("'comail-stalwart-beetmash-com'");
		strategy["route"]["match"]["2"]["then"]
			.as_str()
			.unwrap()
			.xpect_eq("'comail-news-beetmash-com'");
	}

	/// A mixed stack routes each sender domain to its own provider and falls
	/// through to `mx`: local first, then one branch per relayed domain, then
	/// direct delivery for everything else.
	#[beet_core::test]
	fn a_mixed_stack_routes_by_sender_domain() {
		let domains = [staging(), news()];
		let mut relays = RelayModes::default();
		relays.insert("news.beetmash.com", RelayMode::Comail(default()));
		// `staging` declares nothing, so it delivers directly
		let plan = StalwartPlan::new(
			&mail_box(),
			&domains,
			&relays,
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&credentials(&["comail-news-beetmash-com"]),
		)
		.unwrap();
		plan.relays.len().xpect_eq(1);
		let strategy = plan.outbound_strategy();
		strategy["route"]["match"]["0"]["then"]
			.as_str()
			.unwrap()
			.xpect_eq("'local'");
		strategy["route"]["match"]["1"]["if"]
			.as_str()
			.unwrap()
			.xpect_eq("sender_domain == 'news.beetmash.com'");
		// the direct domain has no branch at all, which is what makes it fall
		// through to the box's own MX delivery
		strategy["route"]["match"]["2"].is_null().xpect_true();
		strategy["route"]["else"].as_str().unwrap().xpect_eq("'mx'");
	}

	/// A stack with no relay anywhere declares no route at all and delivers
	/// every message itself, which is what a fresh `<MailDomainBlock/>` means.
	#[beet_core::test]
	fn no_relay_declares_no_route() {
		let domains = [staging()];
		let plan = StalwartPlan::new(
			&mail_box(),
			&domains,
			&RelayModes::default(),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&HashMap::default(),
		)
		.unwrap();
		plan.relays.xpect_eq(Vec::<serde_json::Value>::new());
		plan.outbound_strategy()["route"]["else"]
			.as_str()
			.unwrap()
			.xpect_eq("'mx'");
	}

	/// A route with no credential is a relay the box would authenticate to as
	/// nobody, so it fails naming the route rather than rendering a plan that
	/// 535s on the first message.
	#[beet_core::test]
	fn a_route_without_a_credential_fails() {
		let domains = [news()];
		StalwartPlan::new(
			&mail_box(),
			&domains,
			&all(&domains, RelayMode::Comail(ComailRelay::default())),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&HashMap::default(),
		)
		.unwrap_err()
		.to_string()
		.xpect_contains("no credential for the 'comail-news-beetmash-com'");
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
		let domains = [staging(), news(), apex];
		let plan = StalwartPlan::new(
			&mail_box(),
			&domains,
			&all(&domains, RelayMode::Ses(SesRelay::default())),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&credentials(&[SesRelay::ROUTE]),
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
		let domains = [domain];
		StalwartPlan::new(
			&mail_box(),
			&domains,
			&all(&domains, RelayMode::Ses(SesRelay::default())),
			&stack(),
			"postmaster@stalwart.beetmash.com",
			&credentials(&[SesRelay::ROUTE]),
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
	fn the_relay_password_appears_only_on_the_relay_route() {
		let plan = plan();
		let rendered =
			serde_json::to_string(&plan.domains[0].object("acme1")).unwrap();
		rendered.contains("smtp-derived-password").xpect_false();
		plan.relays[0]["authSecret"]["secret"]
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
		// rewritten by SES on every message
		headers.contains_key("Message-ID").xpect_false();
		headers.contains_key("Date").xpect_false();
		// re-parsed and re-emitted by SES, so a bare angle form is rewritten
		headers.contains_key("To").xpect_false();
		headers.contains_key("Cc").xpect_false();
		headers.contains_key("Reply-To").xpect_false();
		headers["From"].as_bool().unwrap().xpect_true();
		headers["Subject"].as_bool().unwrap().xpect_true();
	}

	/// A receiver is a reporter, and the reporting this box does on the
	/// ecosystem's behalf is the aggregate half only.
	///
	/// REGRESSION: neither singleton was declared, so the server ran its own
	/// defaults: `daily` aggregates (wanted) beside a `[1, 1d]` failure rate
	/// (not). Two SES bounces on 2026-09-02 and 2026-09-03 are what surfaced
	/// it: a spoofed `investing.com` message produced a failure report AND an
	/// aggregate report to an address that bounces both.
	#[beet_core::test]
	fn aggregate_reports_are_sent_and_failure_reports_are_not() {
		let dmarc = plan().dmarc_report_settings();
		dmarc["aggregateSendFrequency"]["else"]
			.as_str()
			.unwrap()
			.xpect_eq("daily");
		dmarc["failureSendFrequency"]["else"]
			.as_str()
			.unwrap()
			.xpect_eq("disable");
	}

	/// The pre-DMARC per-message reports go off the same way, and share one
	/// shape because both singletons name the property `sendFrequency`.
	#[beet_core::test]
	fn the_dkim_and_spf_failure_reports_are_off_too() {
		plan().auth_failure_report_settings()["sendFrequency"]["else"]
			.as_str()
			.unwrap()
			.xpect_eq("disable");
	}

	/// A frequency is an expression CONSTANT, so it carries no quotes, unlike
	/// the string literals in the same position elsewhere in the plan (a
	/// quoted `'disable'` is a string that no constant matches, which would
	/// leave the shipped default in place and report exactly as before).
	#[beet_core::test]
	fn a_frequency_is_an_unquoted_constant() {
		let dmarc = plan().dmarc_report_settings();
		for value in ["aggregateSendFrequency", "failureSendFrequency"] {
			dmarc[value]["else"]
				.as_str()
				.unwrap()
				.contains('\'')
				.xpect_false();
		}
	}

	/// The primary is the FIRST declared domain, and it is what every outbound
	/// report is addressed from and the only certificate order carrying the
	/// box's own hostname. Both are invisible at the tag that decides them,
	/// which is why the entry's tag order is a documented decision rather than
	/// a layout preference.
	#[beet_core::test]
	fn the_first_declared_domain_is_the_primary() {
		let plan = plan();
		plan.domains[0]
			.name
			.as_str()
			.xpect_eq("stalwart.beetmash.com");
		plan.domains[0]
			.certificate_names
			.contains(&"mail.beetmash.com".to_string())
			.xpect_true();
		plan.domains[1]
			.certificate_names
			.contains(&"mail.beetmash.com".to_string())
			.xpect_false();
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
