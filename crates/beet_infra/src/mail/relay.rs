//! Which relay a mail domain's outbound message leaves through, and the
//! provider knobs that answer belongs to.
//!
//! A relay is composed BESIDE a [`MailDomainBlock`] rather than named as a
//! field on it, because there is no field a third provider could be added to:
//! SES needs a configuration set, an event stream and a custom envelope
//! domain; comail needs an enrolled DID and a pair of published selectors;
//! delivering directly needs neither. Each provider's knobs live on its own
//! component, one of which the domain (or an ancestor of it) carries.
use crate::prelude::*;
use beet_core::prelude::*;

/// Amazon SES as a domain's outbound relay: `<MailDomainBlock domain=".."
/// {SesRelay}/>`.
///
/// The relay this stack was built against, and the one whose deliverability is
/// bought rather than earned: mail leaves from addresses Amazon keeps trusted,
/// at the cost of a sending identity per domain, an IAM user holding a static
/// SMTP credential, and a bounce rate somebody else decides is too high.
///
/// Every SES-shaped constant lives here rather than on the domain, so a domain
/// that relays through something else carries none of them.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Get, SetWith, Component, Reflect,
)]
#[reflect(Component, Default)]
pub struct SesRelay {
	/// The SNS topic, by bare name in this account and region, that this
	/// domain's bounces, complaints, rejections and delivery delays publish
	/// to. Without one the configuration set still collects reputation metrics
	/// but the raw event stream goes nowhere.
	///
	/// SES-only, and distinct from
	/// [`alarms_topic`](MailDomainBlock::alarms_topic): that one is where an
	/// alarm fires, which every provider has, while this is a stream of SES
	/// event objects, which only SES emits.
	///
	/// A name rather than a block reference because one topic serves every
	/// domain in the account: events carry the configuration set that emitted
	/// them, so a topic per domain would only move routing into IAM.
	#[set_with(unwrap_option, into)]
	events_topic: Option<SmolStr>,
}

impl SesRelay {
	/// The relay route name every SES-relayed domain sends on. One route for
	/// all of them, because one IAM user sends for every identity in the
	/// account; comail's counterpart is per domain
	/// ([`ComailRelay::route_name`]), because its key pins one.
	pub const ROUTE: &'static str = "ses";

	/// The SPF policy an SES-relayed domain publishes: SES is the only
	/// authorised sender, and `-all` (hard fail) rather than `~all`, because a
	/// soft fail asks receivers to accept forgeries and think about it.
	pub const SPF: &'static str = "v=spf1 include:amazonses.com -all";

	/// The subdomain SES's custom MAIL FROM uses, ie the envelope sender's
	/// domain. Its own `MX` and SPF are what make SPF *aligned* with the header
	/// From, which is one of the two ways to pass DMARC.
	pub const MAIL_FROM_LABEL: &'static str = "bounce";

	/// Easy DKIM publishes three rotating selectors, so three `CNAME`s per
	/// identity. Fixed by SES.
	pub const DKIM_TOKENS: usize = 3;

	/// The sesv2 key-length enum. Note the `_BIT` suffix: the v1 API and most
	/// documentation say `RSA_2048`, which sesv2 rejects.
	pub const DKIM_KEY_LENGTH: &'static str = "RSA_2048_BIT";

	/// The event destination's name, which is the one phase 0B made by hand and
	/// this block now declares: terraform adopts it rather than adding a second
	/// stream beside it.
	pub const EVENT_DESTINATION: &'static str = "sns-events";

	/// The SES events a destination forwards. Deliveries themselves are
	/// deliberately absent: they are the overwhelming majority of the stream
	/// and carry no decision, while these four are each a reason to stop
	/// sending to an address.
	pub const EVENT_TYPES: &'static [&'static str] =
		&["BOUNCE", "COMPLAINT", "REJECT", "DELIVERY_DELAY"];

	/// The CloudWatch namespace the reputation metrics arrive in, published by
	/// the configuration set rather than by anything this stack runs.
	pub const METRIC_NAMESPACE: &'static str = "AWS/SES";

	/// The bounce rate SES itself treats as review-worthy, and the level this
	/// stack alarms at so the warning arrives before the review does.
	pub const BOUNCE_ALARM_RATE: f64 = 0.05;

	/// The complaint rate with the same relationship to a suspension. An order
	/// of magnitude below the bounce threshold, because a complaint is a
	/// recipient saying so rather than a server.
	pub const COMPLAINT_ALARM_RATE: f64 = 0.001;

	/// The SES configuration set a domain sends through, named from
	/// `domain_slug` and NOT stack-composed, so an account's sets read as the
	/// domains they belong to and terraform can adopt one made by hand.
	pub fn configuration_set_name(domain_slug: &str) -> String {
		domain_slug.to_string()
	}

	/// The envelope-sender domain, ie `bounce.stalwart.beetmash.com`.
	pub fn mail_from_domain(domain: &str) -> String {
		format!("{}.{domain}", Self::MAIL_FROM_LABEL)
	}

	/// The SES bounce/complaint feedback host for `region`, the `MX` the
	/// [`mail from domain`](Self::mail_from_domain) points at.
	pub fn feedback_host(region: &str) -> String {
		format!("feedback-smtp.{region}.amazonses.com")
	}

	/// The regional SES SMTP endpoint the relay route submits to, port 587
	/// STARTTLS.
	pub fn smtp_endpoint(stack: &ResolvedStack) -> String {
		format!("email-smtp.{}.amazonaws.com", stack.region())
	}
}

/// [comail](https://comail.at) as a domain's outbound relay:
/// `<MailDomainBlock domain=".." {ComailRelay}/>`.
///
/// An atproto-identity SMTP relay: a domain is enrolled against a DID, and the
/// relay signs its mail with a pair of selectors it publishes for that domain.
/// Nothing about it is provisioned by a deploy (enrolment is OAuth-gated by
/// design), so this component names the endpoint and the deploy VERIFIES what
/// the operator enrolled ([`ComailEnroll`]).
///
/// Two properties shape everything downstream, both of them consequences of the
/// relay being shared rather than rented:
///
/// - The envelope sender is rewritten per recipient to a VERP address at
///   [`relay_domain`](Self::relay_domain), so SPF never ALIGNS and DMARC rides
///   the member-domain DKIM signature. The SES-style custom MAIL FROM
///   subdomain is meaningless here and is not emitted.
/// - The submission gate requires the envelope sender's domain to equal the
///   credential's enrolled domain exactly, so two comail domains need two
///   relay routes with two credentials rather than one shared route.
///
/// The fields exist because the relay is AGPL and self-hostable; a stack
/// pointing them at its own deployment changes nothing else.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
pub struct ComailRelay {
	/// The submission host, which also serves the send and deliverability
	/// APIs.
	#[set_with(into)]
	host: SmolStr,
	/// The relay's own domain: what an SPF `include:` names, what a VERP
	/// envelope sender lives at, and what the smoke sink is addressed on.
	#[set_with(into)]
	relay_domain: SmolStr,
}

impl Default for ComailRelay {
	fn default() -> Self {
		Self {
			host: Self::HOST.into(),
			relay_domain: Self::RELAY_DOMAIN.into(),
		}
	}
}

impl ComailRelay {
	/// The hosted relay's submission host.
	pub const HOST: &'static str = "smtp.atmos.email";

	/// The hosted relay's own domain.
	pub const RELAY_DOMAIN: &'static str = "atmos.email";

	/// The submission port: SASL PLAIN over STARTTLS. There is no implicit-TLS
	/// 465 to fall back to, so the route negotiates or it does not connect.
	pub const SMTP_PORT: i64 = 587;

	/// The localpart at [`relay_domain`](Self::relay_domain) that accepts and
	/// drops, ie the analogue of the SES simulator: a probe reaches it over a
	/// real MX and costs nobody a bounce.
	pub const SINK_LOCALPART: &'static str = "smoke-sink";

	/// The CloudWatch namespace [`ComailDeliverability`] publishes into. Not
	/// `AWS/`-prefixed: that space belongs to AWS's own services and a custom
	/// metric written there is rejected.
	pub const METRIC_NAMESPACE: &'static str = "Comail";

	/// The dimension every metric carries, ie the mail domain the numbers
	/// belong to.
	pub const METRIC_DIMENSION: &'static str = "Domain";

	/// The metric names [`ComailDeliverability`] publishes and the alarms read.
	///
	/// Named here rather than on the action that writes them because the alarms
	/// are emitted by a config-time render that compiles on every target, while
	/// the poller is a native deploy verb: a name only one of the two can see
	/// is a name they can drift on.
	pub const BOUNCE_RATE_METRIC: &'static str = "BounceRate";
	/// Complaints over the same rolling fortnight, as a rate against sends.
	pub const COMPLAINT_RATE_METRIC: &'static str = "ComplaintRate";
	/// Messages accepted over the rolling fortnight, ie the denominator both
	/// rates are meaningless without.
	pub const SENT_METRIC: &'static str = "Sent14d";
	/// `1` while comail has paused or suspended the domain, `0` otherwise: the
	/// state every other metric exists to arrive before.
	pub const PAUSED_METRIC: &'static str = "Paused";

	/// The grant kind a comail domain declares, lowered by
	/// [`IamPolicy::lower`] into a `cloudwatch:PutMetricData` scoped to the
	/// named namespace and nothing else.
	pub const ACCESS_KIND: &'static str = "metric_namespace";

	/// The bounce rate this stack alarms at, deliberately below
	/// [`PAUSE_BOUNCE_RATE`](Self::PAUSE_BOUNCE_RATE) so a warning arrives
	/// before comail acts on the same numbers.
	pub const BOUNCE_ALARM_RATE: f64 = 0.04;

	/// The complaint rate with the same relationship to
	/// [`PAUSE_COMPLAINT_RATE`](Self::PAUSE_COMPLAINT_RATE).
	pub const COMPLAINT_ALARM_RATE: f64 = 0.0005;

	/// The bounce rate comail pauses a domain at, over a rolling 24h window
	/// with at least ten messages
	/// (`internal/relay/domain_pause_evaluator.go:31`).
	///
	/// The shipped configuration, which is what an alarm must beat. A tighter
	/// self-host tier (3% bounce, five messages) exists in the same file and is
	/// gated off behind the paid-tiers kill switch
	/// (`cmd/relay/main.go:477`); if it is ever switched on, both alarm rates
	/// above have to drop below it or the pause arrives first.
	pub const PAUSE_BOUNCE_RATE: f64 = 0.05;

	/// The complaint rate comail pauses at, same window and same switch
	/// (`internal/relay/domain_pause_evaluator.go:32`).
	pub const PAUSE_COMPLAINT_RATE: f64 = 0.0008;

	pub fn new(
		host: impl Into<SmolStr>,
		relay_domain: impl Into<SmolStr>,
	) -> Self {
		Self {
			host: host.into(),
			relay_domain: relay_domain.into(),
		}
	}

	/// The SPF value an enrolled domain publishes at its apex.
	///
	/// `~all` rather than the SES arm's `-all`: the relay's own guidance, and
	/// the honest policy for a sender whose envelope domain is never this one.
	pub fn spf_value(&self) -> String {
		format!("v=spf1 include:{} ~all", self.relay_domain)
	}

	/// The address a probe sends to, ie the accept-and-drop blackhole.
	pub fn sink(&self) -> String {
		format!("{}@{}", Self::SINK_LOCALPART, self.relay_domain)
	}

	/// The relay route one enrolled domain gets, ie `comail-news-example-com`.
	/// One per domain, never one shared: a credential pins the single domain
	/// its session may send from.
	pub fn route_name(domain_slug: &str) -> String {
		format!("comail-{domain_slug}")
	}

	/// Where the operator parks the enrolled DID, ie the SMTP username and the
	/// `X-Atmos-DID` header.
	pub fn did_secret(domain_slug: &str) -> SecretRef {
		SecretRef::new(format!("comail-did-{domain_slug}"))
	}

	/// Where the operator parks the `atmos_…` key, ie the SMTP password and the
	/// bearer token.
	pub fn api_key_secret(domain_slug: &str) -> SecretRef {
		SecretRef::new(format!("comail-api-key-{domain_slug}"))
	}

	/// Where the operator parks the date-based selector stem the enrolment
	/// minted, ie the `atmos20260904` in `atmos20260904r._domainkey`.
	pub fn dkim_selector_secret(domain_slug: &str) -> SecretRef {
		SecretRef::new(format!("comail-dkim-selector-{domain_slug}"))
	}

	/// Where the operator parks the RSA selector's record value.
	pub fn dkim_rsa_secret(domain_slug: &str) -> SecretRef {
		SecretRef::new(format!("comail-dkim-rsa-{domain_slug}"))
	}

	/// Where the operator parks the Ed25519 selector's record value.
	pub fn dkim_ed_secret(domain_slug: &str) -> SecretRef {
		SecretRef::new(format!("comail-dkim-ed-{domain_slug}"))
	}

	/// Every parameter a comail domain reads, as `(secret, what it holds)`, in
	/// the order the enrolment response presents them. One list, so the step
	/// that verifies them and the instructions it prints cannot disagree.
	pub fn secrets(domain_slug: &str) -> Vec<(SecretRef, &'static str)> {
		vec![
			(Self::did_secret(domain_slug), "the enrolled DID"),
			(Self::api_key_secret(domain_slug), "the `atmos_…` api key"),
			(
				Self::dkim_selector_secret(domain_slug),
				"the `dkim.selector` from the enrolment response",
			),
			(
				Self::dkim_rsa_secret(domain_slug),
				"its `dkim.rsaRecord` value",
			),
			(
				Self::dkim_ed_secret(domain_slug),
				"its `dkim.edRecord` value",
			),
		]
	}

	/// The tofu variable a per-domain `suffix` parameter arrives as, ie
	/// `comail_dkim_rsa_news_example_com`. Underscores rather than the slug's
	/// hyphens: `${var.a-b}` is a subtraction in HCL, not a name.
	fn variable(suffix: &str, domain_slug: &str) -> Variable {
		Variable::param(format!(
			"comail_{suffix}_{}",
			domain_slug.replace('-', "_")
		))
	}

	/// The selector stem, which is half of both record NAMES.
	pub fn dkim_selector_variable(domain_slug: &str) -> Variable {
		Self::variable("dkim_selector", domain_slug)
	}

	/// The RSA selector's record value.
	pub fn dkim_rsa_variable(domain_slug: &str) -> Variable {
		Self::variable("dkim_rsa", domain_slug)
	}

	/// The Ed25519 selector's record value.
	pub fn dkim_ed_variable(domain_slug: &str) -> Variable {
		Self::variable("dkim_ed", domain_slug)
	}

	/// The three variables a comail domain hands to the apply, paired with the
	/// parameter each is read out of.
	pub fn dkim_variables(domain_slug: &str) -> Vec<(Variable, SecretRef)> {
		vec![
			(
				Self::dkim_selector_variable(domain_slug),
				Self::dkim_selector_secret(domain_slug),
			),
			(
				Self::dkim_rsa_variable(domain_slug),
				Self::dkim_rsa_secret(domain_slug),
			),
			(
				Self::dkim_ed_variable(domain_slug),
				Self::dkim_ed_secret(domain_slug),
			),
		]
	}

	/// The RSA selector's record name, ie `atmos20260904r._domainkey.<domain>`.
	/// The stem is a terraform reference, since the selector is minted at
	/// enrolment and read out of a parameter like the sovereign key.
	pub fn dkim_rsa_record_name(domain: &str, domain_slug: &str) -> String {
		format!(
			"{}r._domainkey.{domain}",
			Self::dkim_selector_variable(domain_slug).tf_var_ref()
		)
	}

	/// The Ed25519 selector's record name, ie
	/// `atmos20260904e._domainkey.<domain>`.
	pub fn dkim_ed_record_name(domain: &str, domain_slug: &str) -> String {
		format!(
			"{}e._domainkey.{domain}",
			Self::dkim_selector_variable(domain_slug).tf_var_ref()
		)
	}

	/// The deliverability aggregates for `did`, the one member-facing metrics
	/// surface with an api-key path (webhooks are session-cookie registered).
	pub fn deliverability_url(&self, did: &str) -> String {
		format!("https://{}/member/deliverability?did={did}", self.host)
	}

	/// The HTTP send endpoint, ie what a probe's inbound leg posts to.
	pub fn send_url(&self) -> String {
		format!("https://{}/v1/send", self.host)
	}
}

/// The relay one mail domain resolved to, and the only thing every emission,
/// route and probe leg matches on.
///
/// An enum rather than two optional components at each call site, so every
/// match is total and a third provider is one variant plus its arms rather
/// than a new pair of `if let`s wherever a relay is consulted.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum RelayMode {
	/// No relay: the box dials each recipient's MX itself.
	///
	/// The default, and not a degraded one. The stack already publishes the
	/// things that make direct delivery respectable (a stable elastic address
	/// with a matching reverse record, MTA-STS, TLS-RPT, DMARC at `p=reject`
	/// and a signing key nobody else holds), so the trade is that the address's
	/// reputation is yours to build rather than yours to rent.
	#[default]
	None,
	Ses(SesRelay),
	Comail(ComailRelay),
}

impl RelayMode {
	/// The SPF policy a direct-delivering domain publishes: the box is the
	/// domain's own `MX`, so `mx` authorises it with no apply-time value at
	/// all, and `-all` because that really is the whole list.
	pub const DIRECT_SPF: &'static str = "v=spf1 mx -all";

	/// The name this mode is reported by, ie in a route description or a
	/// deploy log.
	pub fn label(&self) -> &'static str {
		match self {
			Self::None => "direct",
			Self::Ses(_) => "ses",
			Self::Comail(_) => "comail",
		}
	}

	/// The SPF value the domain publishes under this mode.
	pub fn spf_value(&self) -> String {
		match self {
			Self::None => Self::DIRECT_SPF.to_string(),
			Self::Ses(_) => SesRelay::SPF.to_string(),
			Self::Comail(comail) => comail.spf_value(),
		}
	}

	/// Compose this relay onto `entity`, ie what
	/// `<MailDomainBlock domain=".." {SesRelay}/>` does in markup.
	///
	/// [`None`](Self::None) inserts nothing, because that is what it means: the
	/// absence of a relay component IS direct delivery.
	pub fn insert(&self, entity: &mut EntityWorldMut) {
		match self {
			Self::None => {}
			Self::Ses(ses) => {
				entity.insert(ses.clone());
			}
			Self::Comail(comail) => {
				entity.insert(comail.clone());
			}
		}
	}

	/// The relay route a domain's outbound mail takes, `None` for direct
	/// delivery (which is not a route but the absence of one).
	pub fn route_name(&self, domain_slug: &str) -> Option<String> {
		match self {
			Self::None => None,
			Self::Ses(_) => Some(SesRelay::ROUTE.to_string()),
			Self::Comail(_) => Some(ComailRelay::route_name(domain_slug)),
		}
	}
}

/// Every mail domain's resolved [`RelayMode`], keyed by domain name.
///
/// Resolved once per deploy and read by everything downstream: the domain's own
/// emission, the box's SES sender, the relay routes and the probe, so no two of
/// them can reach a different answer for the same domain.
#[derive(Debug, Default, Clone)]
pub struct RelayModes(HashMap<SmolStr, RelayMode>);

impl RelayModes {
	/// The mode `domain` resolved to. A domain nothing declared a relay for is
	/// [`RelayMode::None`], which is the default's whole meaning.
	pub fn get(&self, domain: &str) -> &RelayMode {
		const NONE: &RelayMode = &RelayMode::None;
		self.0.get(domain).unwrap_or(NONE)
	}

	pub fn insert(&mut self, domain: impl Into<SmolStr>, mode: RelayMode) {
		self.0.insert(domain.into(), mode);
	}

	/// Whether any domain relays through SES, ie whether the box needs the
	/// sending identity and its static credential at all.
	pub fn any_ses(&self) -> bool {
		self.0
			.values()
			.any(|mode| matches!(mode, RelayMode::Ses(_)))
	}
}

/// Resolving the relay composed beside (or above) a [`MailDomainBlock`].
///
/// Ancestry rather than the entity alone, so one `{SesRelay}` on a `<Stack>`
/// covers every domain declared under it and a domain overrides it by carrying
/// its own, the same shape as `AnalyticsRetention`.
#[derive(SystemParam)]
pub struct RelayQuery<'w, 's> {
	relays: AncestorQuery<
		'w,
		's,
		(Option<&'static SesRelay>, Option<&'static ComailRelay>),
		Or<(With<SesRelay>, With<ComailRelay>)>,
	>,
}

impl RelayQuery<'_, '_> {
	/// The relay `entity` resolves: its own, else the nearest ancestor's, else
	/// none.
	///
	/// Both components on ONE entity is a declaration error rather than a
	/// precedence rule, because there is no reading of "this domain relays
	/// through SES and through comail" that a deploy could act on.
	pub fn resolve(&self, entity: Entity, label: &str) -> Result<RelayMode> {
		match self.relays.get(entity) {
			Err(_) => RelayMode::None,
			Ok((Some(_), Some(_))) => bevybail!(
				"'{label}' declares both a `SesRelay` and a `ComailRelay`: a \
				domain relays through one provider or none, so remove the one \
				it does not send through"
			),
			Ok((Some(ses), None)) => RelayMode::Ses(ses.clone()),
			Ok((None, Some(comail))) => RelayMode::Comail(comail.clone()),
			// `Or` admits nothing carrying neither
			Ok((None, None)) => RelayMode::None,
		}
		.xok()
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The mode `entity` resolves in a world `func` builds, by the same query
	/// the render systems use.
	fn resolve(
		world: &mut World,
		entity: Entity,
	) -> std::result::Result<RelayMode, String> {
		world.with_state::<RelayQuery, _>(|relays| {
			relays
				.resolve(entity, "test")
				.map_err(|err| err.to_string())
		})
	}

	/// A domain's own relay wins, and one on an ancestor covers every domain
	/// under it: the whole point of composing rather than naming a field is
	/// that a stack declares its default once.
	#[beet_core::test]
	fn a_relay_resolves_by_ancestry() {
		let mut world = World::new();
		let mut own = Entity::PLACEHOLDER;
		let mut inherited = Entity::PLACEHOLDER;
		world.spawn(SesRelay::default()).with_children(|parent| {
			own = parent.spawn(ComailRelay::default()).id();
			inherited = parent.spawn(()).id();
		});
		world.flush();
		resolve(&mut world, own)
			.unwrap()
			.xpect_eq(RelayMode::Comail(ComailRelay::default()));
		resolve(&mut world, inherited)
			.unwrap()
			.xpect_eq(RelayMode::Ses(SesRelay::default()));
	}

	/// Absent entirely is direct delivery, which is the default a fresh stack
	/// gets and the one behaviour change of this refactor.
	#[beet_core::test]
	fn no_relay_is_direct_delivery() {
		let mut world = World::new();
		let entity = world.spawn(()).id();
		resolve(&mut world, entity)
			.unwrap()
			.xpect_eq(RelayMode::None);
		RelayMode::None
			.spf_value()
			.as_str()
			.xpect_eq("v=spf1 mx -all");
	}

	/// Both at once is a typo rather than a precedence question, so it fails
	/// naming the entity instead of silently picking one.
	#[beet_core::test]
	fn both_relays_on_one_entity_error() {
		let mut world = World::new();
		let entity = world
			.spawn((SesRelay::default(), ComailRelay::default()))
			.id();
		resolve(&mut world, entity)
			.unwrap_err()
			.as_str()
			.xpect_contains("declares both");
	}

	/// The alarm rates must stay UNDER comail's own auto-pause thresholds, or
	/// the warning arrives after the pause it exists to pre-empt.
	#[beet_core::test]
	fn the_alarms_fire_before_comail_pauses() {
		ComailRelay::BOUNCE_ALARM_RATE
			.xpect_less_than(ComailRelay::PAUSE_BOUNCE_RATE);
		ComailRelay::COMPLAINT_ALARM_RATE
			.xpect_less_or_equal_to(ComailRelay::PAUSE_COMPLAINT_RATE);
	}

	/// The record names are the relay's shape and not ours: two TXT selectors
	/// off one date-based stem, which arrives as a variable because enrolment
	/// mints it.
	#[beet_core::test]
	fn comail_publishes_two_selectors_off_one_stem() {
		ComailRelay::dkim_rsa_record_name(
			"news.example.com",
			"news-example-com",
		)
		.as_str()
		.xpect_eq(
			"${var.comail_dkim_selector_news_example_com}r._domainkey.news.example.com",
		);
		ComailRelay::dkim_ed_record_name(
			"news.example.com",
			"news-example-com",
		)
		.as_str()
		.xpect_contains("e._domainkey.news.example.com");
		ComailRelay::default()
			.spf_value()
			.as_str()
			.xpect_eq("v=spf1 include:atmos.email ~all");
	}
}
