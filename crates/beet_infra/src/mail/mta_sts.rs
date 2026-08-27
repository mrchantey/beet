//! MTA-STS (RFC 8461): the promise that mail to this domain is delivered over
//! authenticated TLS, and the two halves that make the promise checkable.
//!
//! The policy itself is a text file served over HTTPS at a well-known path on a
//! dedicated host, and a `_mta-sts` TXT record carries only an id. A sending MTA
//! fetches the policy once, caches it for `max_age`, and re-fetches when the id
//! changes. Both halves therefore have to move together, which is why the record
//! value and the policy body are generated from the one type.
use beet_core::prelude::*;

/// How strictly a sender should treat a failure to reach this domain over
/// authenticated TLS.
#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Reflect,
)]
#[reflect(Default)]
pub enum MtaStsMode {
	/// Publish the policy and expect senders to report failures, but never to
	/// withhold mail over one. The only safe launch mode: a policy is
	/// advertised long before there is any evidence it is correct, and
	/// `enforce` on a wrong policy silently deletes inbound mail.
	#[default]
	Testing,
	/// Withhold mail rather than deliver it over a connection that fails the
	/// policy. Flip here once TLS-RPT has been clean for a fortnight.
	Enforce,
	/// Cancel a previously published policy without leaving senders to time out
	/// against a host that has gone away.
	None,
}

impl MtaStsMode {
	/// The wire value, ie the `mode:` line of the policy body.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Testing => "testing",
			Self::Enforce => "enforce",
			Self::None => "none",
		}
	}
}

/// The MTA-STS policy a mail domain publishes, and the id its TXT record
/// carries.
#[derive(
	Debug, Clone, PartialEq, Eq, Get, SetWith, Serialize, Deserialize, Reflect,
)]
#[reflect(Default)]
pub struct MtaStsPolicy {
	/// See [`MtaStsMode`].
	mode: MtaStsMode,
	/// How long a sender may cache this policy, in seconds. Long is the point:
	/// a cached policy is what stops an attacker who controls the network from
	/// stripping the domain's TLS on a first contact.
	max_age: u32,
}

impl Default for MtaStsPolicy {
	fn default() -> Self {
		Self {
			mode: MtaStsMode::Testing,
			max_age: Self::DEFAULT_MAX_AGE,
		}
	}
}

impl MtaStsPolicy {
	/// One week, the value RFC 8461 suggests for a policy still being proven.
	/// A mature domain publishes longer.
	pub const DEFAULT_MAX_AGE: u32 = 604800;

	/// The path the policy is served at, fixed by RFC 8461. A sender fetches
	/// exactly this and nothing else, so it is never configuration.
	pub const WELL_KNOWN_PATH: &'static str = ".well-known/mta-sts.txt";

	/// A policy in [`Enforce`](MtaStsMode::Enforce) mode, the fast-follow once
	/// TLS-RPT shows the `testing` policy was right.
	pub fn enforce() -> Self { Self::default().with_mode(MtaStsMode::Enforce) }

	/// The host serving `domain`'s policy, ie `mta-sts.stalwart.beetmash.com`.
	/// Fixed by RFC 8461 in the same way the path is.
	pub fn host(domain: &str) -> String { format!("mta-sts.{domain}") }

	/// The record name carrying `domain`'s policy id, ie
	/// `_mta-sts.stalwart.beetmash.com`.
	pub fn record_name(domain: &str) -> String { format!("_mta-sts.{domain}") }

	/// The full url a sending MTA fetches `domain`'s policy from.
	pub fn policy_url(domain: &str) -> String {
		format!("https://{}/{}", Self::host(domain), Self::WELL_KNOWN_PATH)
	}

	/// The `_mta-sts` TXT value. `id` is opaque to senders and compared only for
	/// change, so a deploy stamp is enough (see [`Self::policy_id`]).
	pub fn record_value(id: &str) -> String { format!("v=STSv1; id={id}") }

	/// A policy id derived from a deploy stamp. RFC 8461 allows at most 32
	/// alphanumerics, so anything else in the stamp is dropped rather than
	/// escaped: the value's only job is to differ from the last one.
	pub fn policy_id(deploy_stamp: &str) -> String {
		deploy_stamp
			.chars()
			.filter(char::is_ascii_alphanumeric)
			.take(32)
			.collect::<String>()
			.xmap(|id| match id.is_empty() {
				true => "0".to_string(),
				false => id,
			})
	}

	/// The policy body served at [`WELL_KNOWN_PATH`](Self::WELL_KNOWN_PATH).
	/// `mx_hosts` are the hostnames a sender may present a certificate for,
	/// which for a single-box stack is just the box.
	///
	/// RFC 8461 lines are CRLF-terminated and the media type is `text/plain`.
	pub fn policy_text(&self, mx_hosts: &[&str]) -> String {
		[
			"version: STSv1".to_string(),
			format!("mode: {}", self.mode.as_str()),
		]
		.into_iter()
		.chain(mx_hosts.iter().map(|host| format!("mx: {host}")))
		.chain([format!("max_age: {}", self.max_age)])
		.map(|line| format!("{line}\r\n"))
		.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// The body is the wire format, so it is pinned: a sender parses it
	/// literally and a stray space or LF-only line ending is a policy that does
	/// not apply.
	#[beet_core::test]
	fn policy_body_is_the_wire_format() {
		MtaStsPolicy::default()
			.policy_text(&["mail.beetmash.com"])
			.as_str()
			.xpect_eq(
				"version: STSv1\r\nmode: testing\r\nmx: mail.beetmash.com\r\nmax_age: 604800\r\n",
			);
	}

	/// A domain behind more than one MX lists them all: a sender matches the
	/// certificate it is offered against any of the patterns.
	#[beet_core::test]
	fn every_mx_host_is_listed() {
		MtaStsPolicy::enforce()
			.policy_text(&["mail.beetmash.com", "mail2.beetmash.com"])
			.as_str()
			.xpect_contains("mode: enforce")
			.xpect_contains("mx: mail.beetmash.com\r\nmx: mail2.beetmash.com");
	}

	/// The url and the record name are both fixed by the rfc, and a sender that
	/// finds the record but not the policy withholds nothing: it simply never
	/// caches one.
	#[beet_core::test]
	fn well_known_locations_are_derived_from_the_domain() {
		MtaStsPolicy::policy_url("stalwart.beetmash.com")
			.as_str()
			.xpect_eq(
				"https://mta-sts.stalwart.beetmash.com/.well-known/mta-sts.txt",
			);
		MtaStsPolicy::record_name("stalwart.beetmash.com")
			.as_str()
			.xpect_eq("_mta-sts.stalwart.beetmash.com");
	}

	/// The id must survive being cut out of a deploy stamp: at most 32
	/// alphanumerics, and never empty (an empty id is a malformed record, which
	/// senders treat as no policy at all).
	#[beet_core::test]
	fn policy_id_is_alphanumeric_and_bounded() {
		MtaStsPolicy::policy_id("1756180000s")
			.as_str()
			.xpect_eq("1756180000s");
		MtaStsPolicy::policy_id("2026-08-26T04:15:00Z")
			.as_str()
			.xpect_eq("20260826T041500Z");
		MtaStsPolicy::policy_id(&"x".repeat(40)).len().xpect_eq(32);
		MtaStsPolicy::policy_id("---").as_str().xpect_eq("0");
	}
}
