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

/// A record's metadata: the plaintext index entry under `[secrets.NAME]`,
/// mirrored inside its group's sealed blob (minus `group`, which the blob
/// implies). The sealed copy is the truth and the index its mirror, so a
/// hand edit of the index is caught on open rather than obeyed.
#[derive(
	Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct SecretRecord {
	/// What consumes the record automatically.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub role: Option<SecretRole>,
	/// The group the record is sealed in; [`DEFAULT_GROUP`](Self::DEFAULT_GROUP)
	/// when absent.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub group: Option<SmolStr>,
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

impl SecretRecord {
	/// The group a record with none named lands in.
	pub const DEFAULT_GROUP: &'static str = "default";

	/// The record's group, `default` when none is named.
	pub fn group(&self) -> &str {
		self.group.as_deref().unwrap_or(Self::DEFAULT_GROUP)
	}

	/// The record with its group set, `None` for the default so the index
	/// stays lean.
	pub fn with_group(mut self, group: impl AsRef<str>) -> Self {
		self.group = Some(group.as_ref())
			.filter(|group| *group != Self::DEFAULT_GROUP)
			.map(SmolStr::new);
		self
	}

	/// The record as it is sealed: without its group.
	pub fn without_group(mut self) -> Self {
		self.group = None;
		self
	}
}

/// One record inside a group's sealed blob: the value and a copy of its
/// index entry. [`Debug`] redacts the value.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedRecord {
	/// The secret itself.
	pub value: SmolStr,
	/// The record's metadata, mirrored from the index without its group.
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
/// (a `.toml.age` document seals toml), so `age -d` on the pasted blob
/// yields a file in the format already chosen.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedGroup {
	/// Who the blob was sealed to, so a list edited since can be told.
	#[serde(default)]
	pub recipients: Vec<AgeRecipient>,
	/// The records, keyed by name.
	#[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
	pub secrets: BTreeMap<SmolStr, SealedRecord>,
}

/// One opened record: its name, its value and its sealed metadata with the
/// group it came from. [`Debug`] redacts the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret {
	/// The record's name: an env var name, or a stack's secret label.
	pub name: SmolStr,
	/// The secret itself.
	pub value: SmolStr,
	/// The sealed metadata, its `group` set to the group it was read from.
	pub record: SecretRecord,
}

impl Secret {
	/// The group the secret was sealed in.
	pub fn group(&self) -> &str { self.record.group() }
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

	#[crate::test]
	fn group_defaults() {
		SecretRecord::default().group().xpect_eq("default");
		SecretRecord::default()
			.with_group("default")
			.group
			.xpect_none();
		SecretRecord::default()
			.with_group("agents")
			.group()
			.xpect_eq("agents");
	}

	#[crate::test]
	fn debug_redacts() {
		let secret = Secret {
			name: "TOKEN".into(),
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
