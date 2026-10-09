//! How a secret is rolled, declared by whatever mints it.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// The declared way a secret is rolled, carried by whatever mints it and
/// stored beside the value (a document record's `roll`, a parameter's
/// description), so a store lists how each of its secrets rolls without
/// the declarations in hand. `secrets/revoke` runs every roll it can and
/// prints the rest as the residue that needs a hand; a mint site cannot
/// omit one, so no secret arrives un-rollable.
///
/// Written as one string: `replace:<resource>`, `remint`, `elevated`,
/// `manual:<why>`. Named for what it rolls, since a bare `Roll` reads as the
/// rotation about a forward axis in a crate full of transforms.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let roll = "replace:aws_iam_access_key.relay"
/// 	.parse::<SecretRoll>()
/// 	.unwrap();
/// roll.kind().xpect_eq("replace");
/// roll.to_string().xpect_eq("replace:aws_iam_access_key.relay");
/// "manual:https://example.com/keys\n> New key"
/// 	.parse::<SecretRoll>()
/// 	.unwrap()
/// 	.xpect_eq(SecretRoll::manual("https://example.com/keys\n> New key"));
/// "roll-somehow".parse::<SecretRoll>().unwrap_err();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum SecretRoll {
	/// A terraform-derived secret (the SES pair, a bucket token): `tofu apply
	/// -replace=<resource>` mints a new one and the same apply re-parks it.
	Replace {
		/// The terraform address, ie `cloudflare_account_token.x`.
		resource: SmolStr,
	},
	/// A create-if-missing value (`<EnsureSecret/>`, a mailbox credential):
	/// delete the entry, and the next `deploy` mints a fresh one and
	/// re-provisions its consumer.
	Remint,
	/// A deploy credential an elevated deploy keeps in line with the
	/// declarations: `<stack>/deploy --elevated --roll` replaces it, which
	/// asks a person for the human factor, so `secrets/revoke` lists it rather
	/// than running it.
	Elevated,
	/// Only a hand rolls it, for `why` (a DKIM key, whose roll is a
	/// new selector beside the published one).
	Manual {
		/// What a hand has to do: the full url first, then one dashboard
		/// step or permission per line.
		why: SmolStr,
	},
}

impl SecretRoll {
	/// A [`Manual`](Self::Manual) roll for `why`.
	pub fn manual(why: impl Into<SmolStr>) -> Self {
		Self::Manual { why: why.into() }
	}

	/// A [`Replace`](Self::Replace) of `resource`.
	pub fn replace(resource: impl Into<SmolStr>) -> Self {
		Self::Replace {
			resource: resource.into(),
		}
	}

	/// The one-word kind, for a ledger: `replace`, `remint`, `elevated`,
	/// `manual`.
	pub fn kind(&self) -> &'static str {
		match self {
			Self::Replace { .. } => "replace",
			Self::Remint => "remint",
			Self::Elevated => "elevated",
			Self::Manual { .. } => "manual",
		}
	}
}

impl fmt::Display for SecretRoll {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Replace { resource } => write!(f, "replace:{resource}"),
			Self::Remint => f.write_str("remint"),
			Self::Elevated => f.write_str("elevated"),
			Self::Manual { why } => write!(f, "manual:{why}"),
		}
	}
}

impl FromStr for SecretRoll {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> {
		let value = value.trim();
		match value.split_once(':') {
			None if value == "remint" => Self::Remint.xok(),
			None if value == "elevated" => Self::Elevated.xok(),
			Some(("replace", resource)) if !resource.trim().is_empty() => {
				Self::replace(resource.trim()).xok()
			}
			Some(("manual", why)) if !why.trim().is_empty() => {
				Self::manual(why.trim()).xok()
			}
			_ => bevybail!(
				"`{value}` is not a roll: `replace:<resource>`, `remint`, \
				`elevated` or `manual:<why>`"
			),
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for SecretRoll {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SecretRoll {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Self, D::Error> {
		let text = <alloc::borrow::Cow<str>>::deserialize(deserializer)?;
		text.parse().map_err(serde::de::Error::custom)
	}
}

#[cfg(all(test, feature = "vault"))]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn roundtrips_the_string_form() {
		for (roll, text) in [
			(SecretRoll::Remint, "remint"),
			(SecretRoll::Elevated, "elevated"),
			(
				SecretRoll::replace("cloudflare_account_token.x"),
				"replace:cloudflare_account_token.x",
			),
			(
				SecretRoll::manual("a new selector: beside the old"),
				"manual:a new selector: beside the old",
			),
		] {
			roll.to_string().xpect_eq(text);
			text.parse::<SecretRoll>().unwrap().xpect_eq(roll.clone());
			// serialized as its string form
			let record = SecretRecord {
				roll: Some(roll.clone()),
				..default()
			};
			let toml = toml::to_string(&record).unwrap();
			toml.xpect_eq(format!("roll = {text:?}\n"));
			toml::from_str::<SecretRecord>(&toml)
				.unwrap()
				.roll
				.xpect_eq(Some(roll));
		}
		// a record sealed before `roll` had its name still opens
		toml::from_str::<SecretRecord>("rotation = \"remint\"\n")
			.unwrap()
			.roll
			.xpect_eq(Some(SecretRoll::Remint));
		"replace:".parse::<SecretRoll>().unwrap_err();
		"weekly".parse::<SecretRoll>().unwrap_err();
		SecretRoll::Remint.kind().xpect_eq("remint");
	}
}
