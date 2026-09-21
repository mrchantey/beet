//! One secret's record: its plaintext index entry, its sealed copy, and the
//! opened form a reader holds.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// What consumes a record automatically. A record with no role is kept and
/// viewed, nothing more; the enum grows a variant only when a second
/// automatic consumer exists in `beet_core`'s own vocabulary.
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum SecretRole {
	/// Loaded into the process environment under the record's name when its
	/// document loads (the process environment wins, then `.env`).
	EnvVar,
}

impl SecretRole {
	/// The spelling `role = ".."` and `--role=..` take.
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::EnvVar => "env_var",
		}
	}
}

impl FromStr for SecretRole {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> {
		match value.trim().to_ascii_lowercase().replace('-', "_").as_str() {
			"env_var" | "envvar" => Self::EnvVar.xok(),
			other => bevybail!(
				"`{other}` is not a secret role: `env_var` loads the record \
				into the process environment, and no role keeps it for viewing"
			),
		}
	}
}

impl fmt::Display for SecretRole {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

/// A record's metadata: the plaintext index entry under
/// `[groups.<group>.secrets.NAME]`, mirrored inside its group's sealed
/// blob. The sealed copy is the truth and the index its mirror, so a hand
/// edit of the index is caught on open rather than obeyed. The group is
/// not a field: it is the map the record sits in.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct SecretRecord {
	/// What consumes the record automatically.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub role: Option<SecretRole>,
	/// A plaintext note for a reader of the index, so it is never a secret:
	/// what the value is for, where it is rotated.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub note: Option<SmolStr>,
	/// When the value was last written.
	#[serde(
		default,
		skip_serializing_if = "Option::is_none",
		with = "iso8601::option"
	)]
	pub modified: Option<Timestamp>,
	/// The provider address the value was exported from, on an export.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub address: Option<SmolStr>,
	/// How the secret is rotated, declared by whatever minted it; absent on
	/// a hand-kept record.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub rotation: Option<SecretRotation>,
}

/// One record inside a group's sealed blob: the value and a copy of its
/// index entry. [`Debug`] redacts the value.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedRecord {
	/// The secret itself.
	pub value: SmolStr,
	/// The record's metadata, mirrored from the index.
	#[serde(flatten)]
	pub record: SecretRecord,
}

impl fmt::Debug for SealedRecord {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("SealedRecord")
			.field("value", &"<redacted>")
			.field("record", &self.record)
			.finish()
	}
}

/// The payload of one group's age blob: the recipient list it was sealed
/// to, and its records by name. Serialized in the document's own media type
/// (a `.toml` document seals toml), so `age -d` on the pasted blob yields
/// a file in the format already chosen.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedGroup {
	/// Who the blob was sealed to, so a list edited since can be told.
	#[serde(default)]
	pub recipients: Vec<AgeRecipient>,
	/// The records, keyed by name.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub secrets: BTreeMap<SmolStr, SealedRecord>,
}

/// One opened record: its name, the group it was sealed in, its value and
/// its sealed metadata. [`Debug`] redacts the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
	/// The record's name: an env var name, or a stack's secret label.
	pub name: SmolStr,
	/// The group the record was sealed in.
	pub group: SmolStr,
	/// The secret itself.
	pub value: SmolStr,
	/// The sealed metadata.
	pub record: SecretRecord,
}

impl Secret {
	/// Unambiguous alphanumerics: no `0`/`O` or `1`/`l`, since a generated
	/// value is read aloud and typed by hand during an incident, and
	/// nothing else, since these values land in shell environment files,
	/// connection strings and a JSON config spliced together on a booting
	/// box: restraint in the alphabet is far cheaper than correct escaping
	/// in every one of those places, and it costs only length to make up
	/// the entropy.
	pub const ALPHABET: &'static [u8] =
		b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

	/// 32 characters of [`ALPHABET`](Self::ALPHABET) is ~185 bits, which is
	/// more than the 128 anything here needs and still fits on one line.
	pub const GENERATED_LENGTH: usize = 32;

	/// Below this a generated value is not worth generating.
	pub const MIN_GENERATED_LENGTH: usize = 16;

	/// A generated value of `length` characters of
	/// [`ALPHABET`](Self::ALPHABET), drawn from the platform entropy source:
	/// how every minted credential and every `set --generate` value is
	/// born. Errors, naming `label`, on a length not worth generating.
	///
	/// ```
	/// # use beet_core::prelude::*;
	/// let value = Secret::generate("db-password", Secret::GENERATED_LENGTH).unwrap();
	/// value.len().xpect_eq(32);
	/// value
	/// 	.bytes()
	/// 	.all(|byte| Secret::ALPHABET.contains(&byte))
	/// 	.xpect_true();
	/// Secret::generate("db-password", 8).unwrap_err();
	/// ```
	pub fn generate(label: &str, length: usize) -> Result<SmolStr> {
		if length < Self::MIN_GENERATED_LENGTH {
			bevybail!(
				"secret `{label}` is {length} characters: too short to be worth \
				generating (at least {})",
				Self::MIN_GENERATED_LENGTH
			);
		}
		let mut source = RandomSource::default();
		(0..length)
			.map(|_| {
				Self::ALPHABET[source.random_range(0..Self::ALPHABET.len())]
					as char
			})
			.collect::<String>()
			.xmap(SmolStr::from)
			.xok()
	}
}

impl fmt::Debug for Secret {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Secret")
			.field("name", &self.name)
			.field("value", &"<redacted>")
			.field("record", &self.record)
			.finish()
	}
}

/// A [`Timestamp`] as ISO 8601 text in a document, so a reviewer reads
/// `modified = "2026-09-18T06:00:00.000Z"` rather than an epoch integer.
pub(crate) mod iso8601 {
	use crate::prelude::*;
	use serde::Deserializer;
	use serde::Serializer;
	use serde::de::Error;

	pub fn serialize<S: Serializer>(
		value: &Timestamp,
		serializer: S,
	) -> core::result::Result<S::Ok, S::Error> {
		serializer.serialize_str(&value.format_iso8601())
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(
		deserializer: D,
	) -> core::result::Result<Timestamp, D::Error> {
		let text = <alloc::borrow::Cow<str>>::deserialize(deserializer)?;
		Timestamp::parse_iso8601(&text).ok_or_else(|| {
			D::Error::custom(format!(
				"`{text}` is not an ISO 8601 UTC timestamp, ie \
				`2026-09-18T06:00:00.000Z`"
			))
		})
	}

	/// The same over an `Option`, `None` never written.
	pub mod option {
		use super::*;

		pub fn serialize<S: Serializer>(
			value: &Option<Timestamp>,
			serializer: S,
		) -> core::result::Result<S::Ok, S::Error> {
			match value {
				Some(value) => super::serialize(value, serializer),
				None => serializer.serialize_none(),
			}
		}

		pub fn deserialize<'de, D: Deserializer<'de>>(
			deserializer: D,
		) -> core::result::Result<Option<Timestamp>, D::Error> {
			super::deserialize(deserializer).map(Some)
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn role_spellings() {
		"env_var"
			.parse::<SecretRole>()
			.unwrap()
			.xpect_eq(SecretRole::EnvVar);
		"EnvVar"
			.parse::<SecretRole>()
			.unwrap()
			.xpect_eq(SecretRole::EnvVar);
		"env-var"
			.parse::<SecretRole>()
			.unwrap()
			.xpect_eq(SecretRole::EnvVar);
		"admin"
			.parse::<SecretRole>()
			.unwrap_err()
			.to_string()
			.xpect_contains("env_var");
		SecretRole::EnvVar.to_string().xpect_eq("env_var");
	}

	/// The alphabet is the whole reason a value is generated here rather than
	/// taken from `openssl rand -base64`: a `/` or a `+` in a password that
	/// gets spliced into a DSN or a shell env file is a bug in a different
	/// file. Two calls must not agree, or the entropy source is not one, and
	/// a length that would not survive being guessed is an error rather than
	/// a weak value nobody notices.
	#[crate::test]
	fn generates_alphanumerics() {
		let value =
			Secret::generate("db-password", Secret::GENERATED_LENGTH).unwrap();
		value.len().xpect_eq(Secret::GENERATED_LENGTH);
		value
			.chars()
			.all(|char| char.is_ascii_alphanumeric())
			.xpect_true();
		(value
			!= Secret::generate("db-password", Secret::GENERATED_LENGTH)
				.unwrap())
		.xpect_true();
		Secret::generate("db-password", 8)
			.unwrap_err()
			.to_string()
			.xpect_contains("too short");
	}

	#[crate::test]
	fn debug_redacts() {
		let secret = Secret {
			name: "TOKEN".into(),
			group: "default".into(),
			value: "hunter2".into(),
			record: default(),
		};
		format!("{secret:?}")
			.xpect_contains("<redacted>")
			.xnot()
			.xpect_contains("hunter2");
		let sealed = SealedRecord {
			value: "hunter2".into(),
			record: default(),
		};
		format!("{sealed:?}").xnot().xpect_contains("hunter2");
	}
}
