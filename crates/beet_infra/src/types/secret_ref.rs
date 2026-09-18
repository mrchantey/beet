//! How a stack's secrets are named.
use crate::prelude::*;
use beet_core::prelude::*;

/// One of a stack's secrets by label, ie `db-password`: the name every end
/// of the reference goes through, whatever store holds the value. The
/// stack's `SecretStore` composes its own address from the pair
/// (`SecretStore::address`), and an export keeps the label so a restore
/// into another provider or region composes a fresh one.
///
/// [`name`](Self::name) is the parameter store composition,
/// `/beetmash/prod/db-password`: the `app--stage--label` with its separators
/// as slashes, which parameter store treats as a directory so every secret
/// of one stack nests under a single prefix and an instance role grants the
/// lot in one statement. It stays here because the terraform-side
/// declarations (a relay pair, a bucket token) and the grants a compute
/// lowers name that parameter, and a block spelling the string itself is
/// exactly the drift this exists to prevent.
#[derive(
	Debug, Default, Clone, Get, Serialize, Deserialize, PartialEq, Eq, Reflect,
)]
pub struct SecretRef {
	/// The secret's label, ie the `db-password` in `/beetmash/prod/db-password`.
	label: SmolStr,
}

impl SecretRef {
	pub fn new(label: impl Into<SmolStr>) -> Self {
		Self {
			label: label.into(),
		}
	}

	/// The full parameter store name, ie `/beetmash/prod/db-password`: the
	/// address `SsmSecretStore` composes, and the one a terraform-declared
	/// secret and a grant name.
	pub fn name(&self, stack: &ResolvedStack) -> String {
		format!(
			"/{}",
			stack.resource_name(self.label.clone()).replace("--", "/")
		)
	}

	/// The `aws_ssm_parameter` resource an apply mints this secret as, for a
	/// value terraform derives (a relay pair, a bucket token): a
	/// `SecureString` at [`name`](Self::name) whose description carries the
	/// note and the rotation ([`description`](Self::description)), so the
	/// store lists it like one an action minted. The rotation is required:
	/// every terraform-minted secret is a [`Rotation::Replace`] of the
	/// resource it derives from, and a mint site cannot omit it.
	pub fn parameter_resource(
		&self,
		stack: &ResolvedStack,
		value: impl Into<SmolStr>,
		note: &str,
		rotation: Rotation,
	) -> serde_json::Value {
		serde_json::json!({
			"name": self.name(stack),
			"type": "SecureString",
			"value": value.into(),
			"description": Self::description(Some(note), Some(&rotation)),
		})
	}

	/// The one string a parameter's description holds: the rotation, then
	/// `::`, then the note, either half optional, so a listing reads both
	/// back ([`parse_description`](Self::parse_description)) and a human
	/// reads it in the console.
	pub fn description(
		note: Option<&str>,
		rotation: Option<&Rotation>,
	) -> String {
		match (rotation, note) {
			(Some(rotation), Some(note)) => format!("{rotation} :: {note}"),
			(Some(rotation), None) => rotation.to_string(),
			(None, Some(note)) => note.to_string(),
			(None, None) => String::new(),
		}
	}

	/// The inverse of [`description`](Self::description): a leading half
	/// that parses as a rotation is one, the rest is the note.
	pub fn parse_description(
		text: &str,
	) -> (Option<SmolStr>, Option<Rotation>) {
		let text = text.trim();
		let note = |text: &str| {
			Some(SmolStr::new(text)).filter(|note| !note.is_empty())
		};
		match text.split_once(" :: ") {
			Some((head, rest))
				if let Ok(rotation) = head.parse::<Rotation>() =>
			{
				(note(rest.trim()), Some(rotation))
			}
			_ => match text.parse::<Rotation>() {
				Ok(rotation) => (None, Some(rotation)),
				Err(_) => (note(text), None),
			},
		}
	}

	/// The directory every secret of `stack` sits under, ie `/beetmash/prod`,
	/// which is what an IAM statement grants with a single `/*`.
	pub fn prefix(stack: &ResolvedStack) -> String {
		// composed from a name rather than from the parts, so the prefix and
		// the names under it cannot disagree about the separator.
		Self::new("x")
			.name(stack)
			.rsplit_once('/')
			.map(|(prefix, _)| prefix.to_string())
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod tests {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn stack() -> ResolvedStack {
		Stack::new("beetmash")
			.with_stage("prod")
			.resolve(&PackageConfig::default())
	}

	/// The composition the live boot scripts and IAM policies already carry, so
	/// the strings are pinned: a renamed parameter is a box that boots without
	/// its database password.
	#[beet_core::test]
	fn composes_a_parameter_directory() {
		SecretRef::new("db-password")
			.name(&stack())
			.xpect_eq("/beetmash/prod/db-password");
		SecretRef::new("mail-admin-password")
			.name(&stack())
			.xpect_eq("/beetmash/prod/mail-admin-password");
	}

	/// A description carries the rotation and the note and reads both back,
	/// each half on its own.
	#[beet_core::test]
	fn descriptions_carry_the_rotation_and_the_note() {
		let replace = Rotation::replace("cloudflare_account_token.x");
		let text = SecretRef::description(Some("r2 token"), Some(&replace));
		text.as_str()
			.xpect_eq("replace:cloudflare_account_token.x :: r2 token");
		SecretRef::parse_description(&text)
			.xpect_eq((Some("r2 token".into()), Some(replace.clone())));
		SecretRef::parse_description("remint")
			.xpect_eq((None, Some(Rotation::Remint)));
		SecretRef::parse_description("just a note")
			.xpect_eq((Some("just a note".into()), None));
		SecretRef::parse_description("").xpect_eq((None, None));
		// a note happening to contain the separator stays a note
		SecretRef::parse_description("weekly :: by hand")
			.xpect_eq((Some("weekly :: by hand".into()), None));
		let resource = SecretRef::new("cold-access-key-id").parameter_resource(
			&stack(),
			"${x.id}",
			"r2 token",
			replace,
		);
		resource["name"]
			.as_str()
			.unwrap()
			.xpect_eq("/beetmash/prod/cold-access-key-id");
		resource["type"].as_str().unwrap().xpect_eq("SecureString");
		resource["description"].as_str().unwrap().xpect_eq(&text);
	}

	/// The prefix is what makes one statement enough, so it must be the parent
	/// of every name and never the name itself.
	#[beet_core::test]
	fn the_prefix_is_the_directory_every_secret_sits_in() {
		let stack = stack();
		let prefix = SecretRef::prefix(&stack);
		prefix.as_str().xpect_eq("/beetmash/prod");
		SecretRef::new("db-password")
			.name(&stack)
			.starts_with(&format!("{prefix}/"))
			.xpect_true();
	}
}
