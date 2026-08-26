//! The declarative identity inputs of the mail stack: who has a mailbox, which
//! addresses redirect into it, and which names are infrastructure rather than
//! anyone's to claim.
//!
//! One [`Member`] declaration is read twice, exactly as a bucket declaration is:
//! the deploy publishes their atproto handle record, and `StalwartProvision`
//! creates their account. Neither restates the other.
use beet_core::prelude::*;

/// Hostnames a [`Member`] may never take, because member handles and
/// infrastructure names share one zone: a member called `mail` would own
/// `mail.beetmash.com`, which is the box.
///
/// Kept deliberately wider than the names in use, since the cost of reserving a
/// name nobody wanted is nothing and the cost of handing out one the stack later
/// needs is a rename of a live identity.
pub const RESERVED_HOSTNAMES: &[&str] = &[
	"admin",
	"api",
	"app",
	"autoconfig",
	"autodiscover",
	"blog",
	"bounce",
	"cdn",
	"dev",
	"docs",
	"imap",
	"jmap",
	"mail",
	"mta-sts",
	"news",
	"pds",
	"smtp",
	"stalwart",
	"staging",
	"static",
	"status",
	"support",
	"www",
];

/// A person or agent with an identity in the zone: a mailbox localpart, the
/// aliases that reach it, and (given a DID) an atproto handle published as
/// `_atproto.<name>.<handle domain>`.
///
/// Members are not assumed human. An agent identity is a member with a mailbox
/// and a handle, which is why nothing here names a person.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith, Serialize, Deserialize)]
pub struct Member {
	/// The handle label and default mailbox localpart, eg `pete`. Validated
	/// against [`RESERVED_HOSTNAMES`] and the DNS label rules, since it becomes
	/// a hostname.
	name: SmolStr,
	/// The atproto DID this member's handle resolves to. Without one no
	/// `_atproto` record is published: the handle is only meaningful once a DID
	/// exists to point it at.
	#[set_with(unwrap_option, into)]
	did: Option<SmolStr>,
}

impl Member {
	/// A member named `name`, with no atproto handle yet.
	pub fn new(name: impl Into<SmolStr>) -> Self {
		Self {
			name: name.into(),
			did: None,
		}
	}

	/// The record name this member's atproto handle is published at, ie
	/// `_atproto.pete.beetmash.com`.
	pub fn handle_record_name(&self, handle_domain: &str) -> String {
		format!("_atproto.{}.{handle_domain}", self.name)
	}

	/// Reject a name that is not a legal DNS label, or that is one of the
	/// [`RESERVED_HOSTNAMES`] the stack itself needs.
	pub fn validate(&self) -> Result {
		validate_dns_label(&self.name, "member name")?;
		if RESERVED_HOSTNAMES.contains(&self.name.as_str()) {
			bevybail!(
				"member name '{}' is a reserved hostname: member handles share the zone with infrastructure names",
				self.name
			);
		}
		Ok(())
	}
}

/// An address on a mail domain that stores mail, ie `pete@stalwart.beetmash.com`.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith, Serialize, Deserialize)]
pub struct Mailbox {
	/// The address localpart, eg `pete`.
	localpart: SmolStr,
	/// Full management API access. Exactly the accounts that administer the
	/// server, which is rarely more than one.
	admin: bool,
	/// The [`Member`] this mailbox belongs to. Role mailboxes (`publications@`,
	/// `probe@`) belong to nobody, which is the point of them.
	#[set_with(unwrap_option, into)]
	member: Option<SmolStr>,
}

impl Mailbox {
	/// A mailbox at `localpart`, belonging to no member and holding no admin
	/// rights.
	pub fn new(localpart: impl Into<SmolStr>) -> Self {
		Self {
			localpart: localpart.into(),
			admin: false,
			member: None,
		}
	}

	/// The mailbox a [`Member`] receives by default: their name as the
	/// localpart, owned by them.
	pub fn for_member(member: &Member) -> Self {
		Self::new(member.name().clone()).with_member(member.name().clone())
	}
}

/// A localpart that delivers into a [`Mailbox`] on the same domain, ie
/// `postmaster@ -> pete@`. An alias stores nothing of its own.
#[derive(Debug, Clone, PartialEq, Eq, Get, SetWith, Serialize, Deserialize)]
pub struct Alias {
	/// The address localpart being aliased, eg `postmaster`.
	localpart: SmolStr,
	/// The localpart of the mailbox it delivers into, eg `pete`.
	target: SmolStr,
}

impl Alias {
	/// An alias from `localpart` to the mailbox at `target`.
	pub fn new(
		localpart: impl Into<SmolStr>,
		target: impl Into<SmolStr>,
	) -> Self {
		Self {
			localpart: localpart.into(),
			target: target.into(),
		}
	}

	/// Localparts an operator is expected to answer at, routed to whoever runs
	/// the server. `postmaster` and `abuse` are the two an outside party
	/// actually tries when something is wrong, and a domain that bounces them
	/// reads as unattended.
	pub const OPERATOR_ROLES: &'static [&'static str] =
		&["postmaster", "abuse", "hostmaster", "admin", "security"];

	/// Localparts the authentication reports are addressed to, kept apart from
	/// the operator roles because they are machine traffic in volume.
	pub const REPORT_ROLES: &'static [&'static str] = &["dmarc", "tlsrpt"];

	/// The full role alias set a served domain publishes from day one:
	/// [`OPERATOR_ROLES`](Self::OPERATOR_ROLES) into `operator`,
	/// [`REPORT_ROLES`](Self::REPORT_ROLES) into `reports`.
	pub fn roles(operator: &str, reports: &str) -> Vec<Self> {
		Self::OPERATOR_ROLES
			.iter()
			.map(|role| Self::new(*role, operator))
			.chain(
				Self::REPORT_ROLES
					.iter()
					.map(|role| Self::new(*role, reports)),
			)
			.collect()
	}
}

/// Reject anything that is not a legal single DNS label: lowercase
/// alphanumerics and inner hyphens, at most 63 characters. `context` names what
/// is being validated, so the error says which declaration to fix.
pub(crate) fn validate_dns_label(label: &str, context: &str) -> Result {
	if label.is_empty() || label.len() > 63 {
		bevybail!("{context} '{label}' must be 1 to 63 characters");
	}
	if label.starts_with('-') || label.ends_with('-') {
		bevybail!("{context} '{label}' must not start or end with a hyphen");
	}
	if let Some(bad) = label.chars().find(|char| {
		!char.is_ascii_lowercase() && !char.is_ascii_digit() && *char != '-'
	}) {
		bevybail!(
			"{context} '{label}' contains '{bad}': only lowercase letters, digits and hyphens are legal in a hostname"
		);
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;

	/// A member name becomes a hostname, so the names the stack needs for itself
	/// are not available: a member called `mail` would own the box's own name.
	#[beet_core::test]
	fn reserved_names_are_rejected() {
		Member::new("pete").validate().unwrap();
		Member::new("mail")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("reserved hostname");
		Member::new("news").validate().unwrap_err();
	}

	/// The same validation catches names that are not legal DNS labels at all,
	/// which would otherwise fail at record-creation time with a provider error
	/// rather than at declaration time.
	#[beet_core::test]
	fn illegal_labels_are_rejected() {
		Member::new("Pete")
			.validate()
			.unwrap_err()
			.to_string()
			.xpect_contains("lowercase");
		Member::new("-pete").validate().unwrap_err();
		Member::new("pete.hayman").validate().unwrap_err();
		Member::new("").validate().unwrap_err();
		Member::new("pete-2").validate().unwrap();
	}

	/// The two role groups land on different mailboxes: a human answers
	/// `postmaster@`, and nobody reads `dmarc@` by hand.
	#[beet_core::test]
	fn roles_split_operator_from_reports() {
		let roles = Alias::roles("pete", "info");
		roles.len().xpect_eq(7);
		roles
			.iter()
			.find(|alias| alias.localpart() == "postmaster")
			.unwrap()
			.target()
			.as_str()
			.xpect_eq("pete");
		roles
			.iter()
			.find(|alias| alias.localpart() == "tlsrpt")
			.unwrap()
			.target()
			.as_str()
			.xpect_eq("info");
	}
}
