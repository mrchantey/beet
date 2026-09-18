//! The public half of an age key pair.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// The public half of an [`AgeIdentity`], `age1..`: what a vault is
/// encrypted to, safe in markup and git. Validated on construction, so a
/// value that exists is one age accepts, and serialized as its string.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let alice = AgeIdentity::generate();
/// let bob = AgeIdentity::generate();
/// let ciphertext = AgeRecipient::encrypt(
/// 	&[alice.to_recipient(), bob.to_recipient()],
/// 	b"for both",
/// )
/// .unwrap();
/// ciphertext.as_str().xpect_starts_with("-----BEGIN AGE ENCRYPTED FILE-----");
/// bob.decrypt(ciphertext.as_bytes()).unwrap().xpect_eq(b"for both".to_vec());
/// ```
#[derive(
	Debug, Clone, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize,
)]
// a dynamic value is validated on the way in, see the `FromReflect` impl
#[reflect(
	from_reflect = false,
	FromReflect,
	Serialize,
	Deserialize,
	PartialEq,
	Hash
)]
#[serde(try_from = "SmolStr", into = "SmolStr")]
pub struct AgeRecipient(SmolStr);

impl AgeRecipient {
	/// What every recipient string starts with.
	pub const PREFIX: &'static str = "age1";

	/// Validate `value` as an age recipient, erroring with where one comes
	/// from when it is not.
	pub fn new(value: impl AsRef<str>) -> Result<Self> {
		let value = value.as_ref().trim();
		value
			.parse::<age::x25519::Recipient>()
			.map(|recipient| Self::new_unchecked(recipient.to_string()))
			.map_err(|err| {
				bevyhow!(
					"`{value}` is not an age recipient ({err}): `age-keygen` \
					prints one starting `{}`, and the identity it prints beside \
					it stays with its human",
					Self::PREFIX
				)
			})
	}

	/// A recipient already known to be valid, ie one derived from an identity.
	pub(crate) fn new_unchecked(value: impl Into<SmolStr>) -> Self {
		Self(value.into())
	}

	/// The `age1..` string.
	pub fn as_str(&self) -> &str { &self.0 }

	/// Encrypt `plaintext` to every recipient, as armored text
	/// (`-----BEGIN AGE ENCRYPTED FILE-----`), the form that survives git,
	/// line-ending tooling and paper. Any one listed identity decrypts it.
	/// Errors on an empty list, naming where a recipient comes from.
	pub fn encrypt(recipients: &[Self], plaintext: &[u8]) -> Result<String> {
		use std::io::Write;
		if recipients.is_empty() {
			bevybail!(
				"no recipients: `age-keygen` prints one starting `{}` per human \
				who may read, and the identity it prints beside it stays with \
				them",
				Self::PREFIX
			);
		}
		let recipients = recipients
			.iter()
			.map(Self::to_age)
			.collect::<Result<Vec<_>>>()?;
		let encryptor = age::Encryptor::with_recipients(
			recipients
				.iter()
				.map(|recipient| recipient as &dyn age::Recipient),
		)?;
		let mut ciphertext = Vec::new();
		let mut writer =
			encryptor.wrap_output(age::armor::ArmoredWriter::wrap_output(
				&mut ciphertext,
				age::armor::Format::AsciiArmor,
			)?)?;
		writer.write_all(plaintext)?;
		writer.finish()?.finish()?;
		String::from_utf8(ciphertext)?.xok()
	}

	/// The `age` crate's recipient, for the encryptor.
	fn to_age(&self) -> Result<age::x25519::Recipient> {
		self.0.parse::<age::x25519::Recipient>().map_err(|err| {
			bevyhow!("`{}` is not an age recipient ({err})", self.0)
		})
	}
}

impl FromStr for AgeRecipient {
	type Err = BevyError;
	fn from_str(value: &str) -> Result<Self> { Self::new(value) }
}

impl TryFrom<SmolStr> for AgeRecipient {
	type Error = BevyError;
	fn try_from(value: SmolStr) -> Result<Self> { Self::new(value) }
}

impl From<AgeRecipient> for SmolStr {
	fn from(recipient: AgeRecipient) -> Self { recipient.0 }
}

impl AsRef<str> for AgeRecipient {
	fn as_ref(&self) -> &str { &self.0 }
}

impl fmt::Display for AgeRecipient {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.0)
	}
}

/// Validates the string a dynamic value carries, so a reflected recipient is
/// as trustworthy as a constructed one; a concrete value is cloned as is.
impl FromReflect for AgeRecipient {
	fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
		if let Some(value) = reflect.try_downcast_ref::<Self>() {
			return Some(value.clone());
		}
		match reflect.reflect_ref() {
			bevy_reflect::ReflectRef::TupleStruct(tuple) => {
				<SmolStr as FromReflect>::from_reflect(tuple.field(0)?)
					.and_then(|value| Self::new(value).ok())
			}
			_ => None,
		}
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	const ARMOR_HEADER: &str = "-----BEGIN AGE ENCRYPTED FILE-----";

	#[crate::test]
	fn single_recipient_roundtrip() {
		let identity = AgeIdentity::generate();
		let ciphertext =
			AgeRecipient::encrypt(&[identity.to_recipient()], b"hello")
				.unwrap();
		ciphertext.as_str().xpect_starts_with(ARMOR_HEADER);
		identity
			.decrypt(ciphertext.as_bytes())
			.unwrap()
			.xpect_eq(b"hello".to_vec());
	}

	#[crate::test]
	fn every_listed_identity_decrypts() {
		let alice = AgeIdentity::generate();
		let bob = AgeIdentity::generate();
		let ciphertext = AgeRecipient::encrypt(
			&[alice.to_recipient(), bob.to_recipient()],
			b"for both",
		)
		.unwrap();
		alice
			.decrypt(ciphertext.as_bytes())
			.unwrap()
			.xpect_eq(b"for both".to_vec());
		bob.decrypt(ciphertext.as_bytes())
			.unwrap()
			.xpect_eq(b"for both".to_vec());
	}

	#[crate::test]
	fn an_unlisted_identity_fails() {
		let alice = AgeIdentity::generate();
		let mallory = AgeIdentity::generate();
		let ciphertext =
			AgeRecipient::encrypt(&[alice.to_recipient()], b"for alice")
				.unwrap();
		mallory
			.decrypt(ciphertext.as_bytes())
			.unwrap_err()
			.to_string()
			.xpect_contains("other recipients");
	}

	#[crate::test]
	fn rejects_a_non_age1_string() {
		AgeRecipient::new("ssh-ed25519 AAAAC3Nza")
			.unwrap_err()
			.to_string()
			.xpect_contains("age-keygen");
	}

	#[crate::test]
	fn refuses_an_empty_list() {
		AgeRecipient::encrypt(&[], b"nobody")
			.unwrap_err()
			.to_string()
			.xpect_contains("age-keygen");
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn serde_is_the_string() {
		let recipient = AgeIdentity::generate().to_recipient();
		let json = serde_json::to_string(&recipient).unwrap();
		json.xpect_eq(format!("\"{recipient}\""));
		serde_json::from_str::<AgeRecipient>(&json)
			.unwrap()
			.xpect_eq(recipient);
		serde_json::from_str::<AgeRecipient>("\"age1nope\"").unwrap_err();
	}

	#[crate::test]
	fn from_reflect_validates() {
		let recipient = AgeIdentity::generate().to_recipient();
		let mut dynamic =
			bevy_reflect::tuple_struct::DynamicTupleStruct::default();
		dynamic.insert(SmolStr::new(recipient.as_str()));
		<AgeRecipient as FromReflect>::from_reflect(&dynamic)
			.unwrap()
			.xpect_eq(recipient);
		let mut dynamic =
			bevy_reflect::tuple_struct::DynamicTupleStruct::default();
		dynamic.insert(SmolStr::new("age1nope"));
		<AgeRecipient as FromReflect>::from_reflect(&dynamic).xpect_none();
	}
}
