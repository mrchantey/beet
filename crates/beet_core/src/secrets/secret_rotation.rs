//! How a secret is rotated, declared by whatever mints it.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// The declared way a secret is rotated, carried by whatever mints it and
/// stored beside the value (a document record's `rotation`, a parameter's
/// description), so a store lists how each of its secrets rotates without
/// the declarations in hand. `secrets/revoke` runs every rotation it can and
/// prints the rest as the residue that needs a hand; a mint site cannot
/// omit one, so no secret arrives un-rotatable.
///
/// Written as one string: `replace:<resource>`, `remint`, `manual:<why>`.
/// Named for what it rotates, since a bare `Rotation` is a 3D thing in the
/// same prelude.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
pub enum SecretRotation {
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
	/// Only a hand rotates it, for `why` (a DKIM key, whose rotation is a
	/// new selector beside the published one).
	Manual {
		/// What a hand has to do, one line.
		why: SmolStr,
	},
}

impl SecretRotation {
	/// A [`Manual`](Self::Manual) rotation for `why`.
	pub fn manual(why: impl Into<SmolStr>) -> Self {
		Self::Manual { why: why.into() }
	}

	/// A [`Replace`](Self::Replace) of `resource`.
	pub fn replace(resource: impl Into<SmolStr>) -> Self {
		Self::Replace {
			resource: resource.into(),
		}
	}

	/// The one-word kind, for a ledger: `replace`, `remint`, `manual`.
	pub fn kind(&self) -> &'static str {
		match self {
			Self::Replace { .. } => "replace",
			Self::Remint => "remint",
			Self::Manual { .. } => "manual",
		}
	}
}

impl fmt::Display for SecretRotation {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Replace { resource } => write!(f, "replace:{resource}"),
			Self::Remint => f.write_str("remint"),
			Self::Manual { why } => write!(f, "manual:{why}"),
		}
	}
}

impl FromStr for SecretRotation {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> {
		let value = value.trim();
		match value.split_once(':') {
			None if value == "remint" => Self::Remint.xok(),
			Some(("replace", resource)) if !resource.trim().is_empty() => {
				Self::replace(resource.trim()).xok()
			}
			Some(("manual", why)) if !why.trim().is_empty() => {
				Self::manual(why.trim()).xok()
			}
			_ => bevybail!(
				"`{value}` is not a rotation: `replace:<resource>`, `remint` or \
				`manual:<why>`"
			),
		}
	}
}

#[cfg(feature = "serde")]
impl Serialize for SecretRotation {
	fn serialize<S: serde::Serializer>(
		&self,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&self.to_string())
	}
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for SecretRotation {
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
		for (rotation, text) in [
			(SecretRotation::Remint, "remint"),
			(
				SecretRotation::replace("cloudflare_account_token.x"),
				"replace:cloudflare_account_token.x",
			),
			(
				SecretRotation::manual("a new selector: beside the old"),
				"manual:a new selector: beside the old",
			),
		] {
			rotation.to_string().xpect_eq(text);
			text.parse::<SecretRotation>()
				.unwrap()
				.xpect_eq(rotation.clone());
			// serialized as its string form
			let record = SecretRecord {
				rotation: Some(rotation.clone()),
				..default()
			};
			let toml = toml::to_string(&record).unwrap();
			toml.xpect_eq(format!("rotation = {text:?}\n"));
			toml::from_str::<SecretRecord>(&toml)
				.unwrap()
				.rotation
				.xpect_eq(Some(rotation));
		}
		"replace:".parse::<SecretRotation>().unwrap_err();
		"weekly".parse::<SecretRotation>().unwrap_err();
		SecretRotation::Remint.kind().xpect_eq("remint");
	}
}
