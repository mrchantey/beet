//! What the zone holds, against what the stack declared.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use serde_json::Value;

/// `<ZoneAudit/>` — assert that the Cloudflare zone contains exactly the
/// records this stack declares, plus the ones it was told to expect.
///
/// Terraform converges the records it owns and is blind to everything else, so
/// a record created by hand during an incident, or left behind by a provider
/// that was migrated away from, sits in the zone indefinitely. For most stacks
/// that is untidy; for mail it is an outage waiting to happen, because a
/// leftover `MX` or a second `SPF` record at one name is not merged by
/// receivers, it is a permanent error.
///
/// The declared set is read from the config the blocks emit rather than
/// restated here, so a block that adds a record does not also have to be added
/// to an audit. Records whose value is only known after apply (the SES DKIM
/// selectors, whose tokens are computed) are matched as patterns on the part of
/// the name that IS known.
///
/// Reports by default and deletes only when asked, since the audit's own
/// allowlist is the thing most likely to be wrong the first time it runs.
#[derive(Debug, Default, Clone, Get, SetWith, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ZoneAuditAction)]
pub struct ZoneAudit {
	/// Records the zone is expected to carry that this stack does not declare:
	/// another stack's, a third party's, or a provider's own.
	///
	/// Every entry is a decision with a reason, which is why they are declared
	/// rather than inferred. The apex mail records of a provider still serving
	/// the domain belong here until its cutover; so does the record wrangler
	/// creates for a worker custom domain, which terraform never sees.
	///
	/// The list belongs to the STACK rather than to the verb that reads it: an
	/// audit run at the tail of a deploy and one run on its own are asking the
	/// same question of the same zone, and a second copy of these rows is a
	/// second thing to update when the cutover retires them. So the action
	/// gathers every `ZoneAudit` declared under the stack, and one of them
	/// carrying the list is enough for all of them.
	#[set_with(skip)]
	allowed: Vec<AllowedRecord>,
}

/// One record the audit expects to find without this stack declaring it.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Get, Serialize, Deserialize, Reflect,
)]
#[reflect(Default)]
pub struct AllowedRecord {
	/// The record name, ie `beetmash.com`. A leading `*.` matches any label in
	/// that position, which is how a computed selector is allowed without
	/// pinning the token.
	name: SmolStr,
	/// The record type, or empty for every type at this name.
	record_type: SmolStr,
	/// Why it is allowed, printed beside it so an audit reads as an inventory
	/// rather than as a list of exceptions.
	reason: SmolStr,
}

impl AllowedRecord {
	pub fn new(
		name: impl Into<SmolStr>,
		record_type: impl Into<SmolStr>,
		reason: impl Into<SmolStr>,
	) -> Self {
		Self {
			name: name.into(),
			record_type: record_type.into(),
			reason: reason.into(),
		}
	}

	fn matches(&self, name: &str, record_type: &str) -> bool {
		if !self.record_type.is_empty() && self.record_type != record_type {
			return false;
		}
		name_matches(&self.name, name)
	}
}

impl ZoneAudit {
	/// Allow one record.
	pub fn with_allowed(
		mut self,
		name: impl Into<SmolStr>,
		record_type: impl Into<SmolStr>,
		reason: impl Into<SmolStr>,
	) -> Self {
		self.allowed
			.push(AllowedRecord::new(name, record_type, reason));
		self
	}

	/// Allow every record at `name`, whatever its type. For a name owned
	/// wholesale by something else, ie a worker's custom domain.
	pub fn with_allowed_name(
		self,
		name: impl Into<SmolStr>,
		reason: impl Into<SmolStr>,
	) -> Self {
		self.with_allowed(name, "", reason)
	}

	/// Allow the records a mail provider serving `domain`'s apex owns: its
	/// exchangers, the `SPF` that authorises them, and its DKIM selectors.
	///
	/// The set every hosted-mail provider publishes, named once here because an
	/// audit that screams about live third-party mail is an audit nobody runs.
	/// It retires with the cutover that replaces them.
	pub fn with_third_party_mail(
		self,
		domain: &str,
		reason: impl Into<SmolStr> + Clone,
	) -> Self {
		self.with_allowed(domain, "MX", reason.clone())
			.with_allowed(domain, "TXT", reason.clone())
			.with_allowed(format!("*._domainkey.{domain}"), "CNAME", reason)
	}
}

/// Enumerates the zone, diffs it and reports. `--fix` deletes the strays.
#[action(handler_only)]
#[derive(Default, Component, Reflect)]
#[reflect(Component, Default)]
#[require(ParamsPartial = ParamsPartial::new::<ZoneAuditParams>())]
pub async fn ZoneAuditAction(
	cx: ActionContext<Request>,
) -> Result<Outcome<Request, Response>> {
	let fix = cx.has_param("fix");
	let (declared, allowed) = cx
		.caller
		.with_state::<(StackQuery, Query<&ZoneAudit>), _>(
			|entity, (stacks, audits)| -> Result<_> {
				let (_, _, config) = stacks.build_config(entity)?;
				// every audit declared under this stack, so the list is stated
				// once wherever it reads best
				let allowed = stacks
					.declared(entity)?
					.into_iter()
					.filter_map(|child| audits.get(child).ok())
					.flat_map(|audit| audit.allowed().iter().cloned())
					.collect::<Vec<_>>();
				(declared_records(&config), allowed).xok()
			},
		)
		.await??;

	let (zone_id, token) = zone_env()?;
	let live = list_records(&zone_id, &token).await?;
	info!(
		"zone holds {} record(s); the stack declares {}",
		live.len(),
		declared.len()
	);

	let strays = live
		.iter()
		.filter(|record| {
			!declared
				.iter()
				.any(|pattern| pattern.matches(&record.name, &record.kind))
				&& !allowed
					.iter()
					.any(|allowed| allowed.matches(&record.name, &record.kind))
		})
		.collect::<Vec<_>>();

	for allowed in &allowed {
		let count = live
			.iter()
			.filter(|record| allowed.matches(&record.name, &record.kind))
			.count();
		info!(
			"allowed: {} {} ({}) - {count} present",
			allowed.record_type(),
			allowed.name(),
			allowed.reason()
		);
	}

	if strays.is_empty() {
		info!("zone is clean: nothing present that is not declared or allowed");
		return Pass(cx.input).xok();
	}
	for stray in &strays {
		error!("stray: {} {} -> {}", stray.kind, stray.name, stray.content);
	}
	if !fix {
		bevybail!(
			"{} stray record(s) in the zone. Each is either a record a block \
			should declare, an entry the audit's allowlist should carry, or a \
			leftover to delete with `--fix`.",
			strays.len()
		);
	}
	for stray in &strays {
		delete_record(&zone_id, &token, &stray.id).await?;
		info!("deleted {} {}", stray.kind, stray.name);
	}
	Pass(cx.input).xok()
}

/// Parameters for the audit.
#[derive(Reflect)]
struct ZoneAuditParams {
	/// Delete the stray records instead of only reporting them.
	fix: bool,
}

/// One record as the zone holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ZoneRecord {
	id: String,
	name: String,
	kind: String,
	content: String,
}

/// One record as the stack declares it: a name that may carry an interpolation,
/// and a type.
#[derive(Debug, Clone, PartialEq, Eq)]
struct DeclaredRecord {
	name: String,
	kind: String,
}

impl DeclaredRecord {
	fn matches(&self, name: &str, kind: &str) -> bool {
		self.kind == kind && name_matches(&self.name, name)
	}
}

/// Every Cloudflare record the config emits.
///
/// Read off the emitted config rather than restated, so the audit and the apply
/// cannot disagree about what this stack owns: a block that adds a record is
/// audited the moment it is declared, with nothing else to remember.
fn declared_records(config: &terra::Config) -> Vec<DeclaredRecord> {
	config.to_json()["resource"]["cloudflare_dns_record"]
		.as_object()
		.map(|records| {
			records
				.values()
				.filter_map(|record| {
					Some(DeclaredRecord {
						name: to_pattern(record["name"].as_str()?),
						kind: record["type"].as_str()?.to_string(),
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

/// Turn a declared name into a match pattern: an interpolation whose value is
/// only known after apply (a computed DKIM selector) becomes a `*` label.
///
/// The alternative is reading the applied state to resolve them, which would
/// make the audit useless before the first apply, which is exactly when a stray
/// record does the most damage.
fn to_pattern(name: &str) -> String {
	let mut pattern = String::with_capacity(name.len());
	let mut rest = name;
	while let Some(start) = rest.find("${") {
		pattern.push_str(&rest[..start]);
		pattern.push('*');
		match rest[start..].find('}') {
			Some(end) => rest = &rest[start + end + 1..],
			None => return pattern,
		}
	}
	pattern.push_str(rest);
	pattern
}

/// Whether `name` matches `pattern`, where `*` stands for any run of characters
/// inside one label.
fn name_matches(pattern: &str, name: &str) -> bool {
	let pattern = pattern.trim_end_matches('.').to_ascii_lowercase();
	let name = name.trim_end_matches('.').to_ascii_lowercase();
	if !pattern.contains('*') {
		return pattern == name;
	}
	let mut cursor = 0usize;
	let segments = pattern.split('*').collect::<Vec<_>>();
	for (index, segment) in segments.iter().enumerate() {
		if segment.is_empty() {
			continue;
		}
		let found = match index {
			0 => name.starts_with(segment).then_some(0),
			_ => name[cursor..].find(segment).map(|at| cursor + at),
		};
		let Some(at) = found else { return false };
		cursor = at + segment.len();
	}
	// a trailing `*` may absorb the rest; anything else must have consumed it
	match segments.last() {
		Some(last) if last.is_empty() => true,
		_ => cursor == name.len(),
	}
}

/// The zone id + api token from the environment, the auth every zone call
/// needs.
fn zone_env() -> Result<(SmolStr, SmolStr)> {
	let zone_id = env_ext::var("CLOUDFLARE_ZONE_ID")
		.map_err(|_| bevyhow!("CLOUDFLARE_ZONE_ID is unset"))?;
	let token = env_ext::var("CLOUDFLARE_API_TOKEN")
		.map_err(|_| bevyhow!("CLOUDFLARE_API_TOKEN is unset"))?;
	Ok((zone_id, token))
}

/// Every record in the zone, paged through to the end. Paging is not optional:
/// the default page is 100 and a zone that outgrew it would report every record
/// past the first page as absent and every one in it as fine.
async fn list_records(zone_id: &str, token: &str) -> Result<Vec<ZoneRecord>> {
	const PER_PAGE: usize = 100;
	let mut records = Vec::new();
	for page in 1.. {
		let response = Request::get(format!(
			"https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records?per_page={PER_PAGE}&page={page}"
		))
		.with_auth_bearer(token)
		.send()
		.await?;
		let status = response.status();
		let body = response.text().await.unwrap_or_default();
		let json: Value = serde_json::from_str(&body).unwrap_or_default();
		if !status.is_ok() || json["success"] != true {
			bevybail!("listing zone {zone_id} failed: {status} - {body}");
		}
		let page_records = json["result"]
			.as_array()
			.cloned()
			.unwrap_or_default()
			.iter()
			.map(|record| ZoneRecord {
				id: record["id"].as_str().unwrap_or_default().to_string(),
				name: record["name"].as_str().unwrap_or_default().to_string(),
				kind: record["type"].as_str().unwrap_or_default().to_string(),
				content: record["content"]
					.as_str()
					.unwrap_or_default()
					.to_string(),
			})
			.collect::<Vec<_>>();
		let complete = page_records.len() < PER_PAGE;
		records.extend(page_records);
		if complete {
			break;
		}
	}
	Ok(records)
}

async fn delete_record(zone_id: &str, token: &str, id: &str) -> Result {
	let response = Request::delete(format!(
		"https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records/{id}"
	))
	.with_auth_bearer(token)
	.send()
	.await?;
	let status = response.status();
	if !status.is_ok() {
		let body = response.text().await.unwrap_or_default();
		bevybail!("deleting record {id} failed: {status} - {body}");
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	fn staging() -> MailDomainBlock {
		MailDomainBlock::new("stalwart.beetmash.com", "mail.beetmash.com")
			.with_dns(DnsProvider::cloudflare("stalwart.beetmash.com", "zone1"))
			.with_mailbox(Mailbox::new("pete"))
	}

	fn declared() -> Vec<DeclaredRecord> {
		let (stack, deployment, _dir) = ResolvedStack::default_local();
		let mut world = World::new();
		let spawned = world.spawn(());
		let entity = spawned.as_readonly();
		let block = staging();
		let config = stack
			.build_config(&deployment, [(entity, &block as &dyn Block)])
			.unwrap();
		declared_records(&config)
	}

	/// The declared set comes off the emitted config, so a block that adds a
	/// record is audited without the audit being edited.
	#[beet_core::test]
	fn the_declared_set_is_read_from_the_config() {
		let declared = declared();
		let matches = |name: &str, kind: &str| {
			declared.iter().any(|record| record.matches(name, kind))
		};
		matches("stalwart.beetmash.com", "MX").xpect_true();
		matches("stalwart.beetmash.com", "TXT").xpect_true();
		matches("_dmarc.stalwart.beetmash.com", "TXT").xpect_true();
		matches("_jmap._tcp.stalwart.beetmash.com", "SRV").xpect_true();
		// never the apex: a stack that declared one would be taking mail off
		// whoever serves it today.
		matches("beetmash.com", "MX").xpect_false();
	}

	/// A DKIM selector's token is computed by SES and unknown until apply, so
	/// the name is matched on the part that IS known. Without this the audit is
	/// useless before the first apply, which is when a stray record is most
	/// dangerous.
	#[beet_core::test]
	fn computed_selectors_match_as_patterns() {
		let declared = declared();
		declared
			.iter()
			.any(|record| {
				record.matches(
					"abcdef123._domainkey.stalwart.beetmash.com",
					"CNAME",
				)
			})
			.xpect_true();
		// and the pattern is a label, not a free-for-all across the zone
		declared
			.iter()
			.any(|record| record.matches("anything.beetmash.com", "CNAME"))
			.xpect_false();
	}

	/// An interpolation collapses to a wildcard wherever it appears, including
	/// several in one name.
	#[beet_core::test]
	fn interpolations_become_wildcards() {
		to_pattern("${a}._domainkey.x.com").xpect_eq("*._domainkey.x.com");
		to_pattern("plain.x.com").xpect_eq("plain.x.com");
		to_pattern("${a}.${b}.x.com").xpect_eq("*.*.x.com");
	}

	/// Matching is case- and trailing-dot-insensitive, since Cloudflare returns
	/// names in its own normalisation and a declaration writes them in ours.
	#[beet_core::test]
	fn names_match_regardless_of_case_or_root_label() {
		name_matches("mail.beetmash.com", "MAIL.beetmash.com.").xpect_true();
		name_matches("mail.beetmash.com", "other.beetmash.com").xpect_false();
	}

	/// The allowlist is what keeps an audit during staging silent rather than
	/// screaming about the third party's live mail. It is scoped: their apex
	/// records are allowed, a stray one at a name they do not own is not.
	#[beet_core::test]
	fn third_party_mail_is_allowed_only_where_it_lives() {
		let audit = ZoneAudit::default().with_third_party_mail(
			"beetmash.com",
			"fastmail, live until cutover",
		);
		let allows = |name: &str, kind: &str| {
			audit
				.allowed()
				.iter()
				.any(|allowed| allowed.matches(name, kind))
		};
		allows("beetmash.com", "MX").xpect_true();
		allows("beetmash.com", "TXT").xpect_true();
		allows("fm1._domainkey.beetmash.com", "CNAME").xpect_true();
		// their selectors, not a licence for every cname in the zone
		allows("dev.beetmash.com", "CNAME").xpect_false();
		// and not our own mail domain's records, which the stack must declare
		allows("stalwart.beetmash.com", "MX").xpect_false();
	}

	/// A name owned wholesale by something terraform never sees (a worker's
	/// custom domain, provisioned by wrangler with its own certificate) is
	/// allowed by name rather than by type.
	#[beet_core::test]
	fn a_name_can_be_allowed_whatever_its_type() {
		let audit = ZoneAudit::default().with_allowed_name(
			"mta-sts.stalwart.beetmash.com",
			"wrangler custom domain",
		);
		let allowed = &audit.allowed()[0];
		allowed
			.matches("mta-sts.stalwart.beetmash.com", "CNAME")
			.xpect_true();
		allowed
			.matches("mta-sts.stalwart.beetmash.com", "AAAA")
			.xpect_true();
		allowed
			.matches("mta-sts.news.beetmash.com", "CNAME")
			.xpect_false();
	}
}
