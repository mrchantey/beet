//! The public half of an age key pair.

use crate::prelude::*;
use core::fmt;
use core::str::FromStr;

/// The public half of an [`AgeIdentity`](crate::prelude::AgeIdentity),
/// `age1..`: what a vault is encrypted to, safe in markup and git. Validated
/// on construction (bech32, the `age` prefix, a 32 byte key, the grammar the
/// age crate reads), so a value that exists is one age accepts, and
/// serialized as its string.
///
/// The type rides every std build, since a `<Vault>` naming its recipients is
/// data a lean binary still loads; [`encrypt`](Self::encrypt) rides the
/// `secrets` feature.
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Reflect)]
// a dynamic value is validated on the way in, see the `FromReflect` impl
#[reflect(from_reflect = false, FromReflect, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "SmolStr", into = "SmolStr"))]
pub struct AgeRecipient(SmolStr);

impl AgeRecipient {
	/// What every recipient string starts with: the bech32 human-readable
	/// part and its separator.
	pub const PREFIX: &'static str = "age1";
	/// The bech32 human-readable part of a recipient.
	const HRP: &'static str = "age";
	/// An x25519 public key.
	const KEY_LEN: usize = 32;

	/// Validate `value` as an age recipient, erroring with where one comes
	/// from when it is not. Stored lowercase, the form age prints.
	pub fn new(value: impl AsRef<str>) -> Result<Self> {
		let value = value.as_ref().trim();
		Self::validate(value)
			.map(|_| Self::new_unchecked(value.to_lowercase()))
			.map_err(|err| {
				bevyhow!(
					"`{value}` is not an age recipient ({err}): `age-keygen` \
					prints one starting `{}`, and the identity it prints beside \
					it stays with its human",
					Self::PREFIX
				)
			})
	}

	/// The grammar age's own parser applies: bech32 (not bech32m), the `age`
	/// prefix, and a 32 byte payload.
	fn validate(value: &str) -> Result<(), &'static str> {
		use bech32::FromBase32;
		let (hrp, data, variant) =
			bech32::decode(value).map_err(|_| "invalid bech32 encoding")?;
		if hrp != Self::HRP || variant != bech32::Variant::Bech32 {
			return Err("incorrect prefix");
		}
		let bytes = Vec::<u8>::from_base32(&data)
			.map_err(|_| "invalid bech32 encoding")?;
		match bytes.len() == Self::KEY_LEN {
			true => Ok(()),
			false => Err("incorrect key length"),
		}
	}

	/// A recipient already known to be valid, ie one derived from an identity.
	pub(crate) fn new_unchecked(value: impl Into<SmolStr>) -> Self {
		Self(value.into())
	}

	/// The `age1..` string.
	pub fn as_str(&self) -> &str { &self.0 }
}

#[cfg(feature = "secrets")]
impl AgeRecipient {
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

	/// A recipient with no identity behind it, for the grammar tests that
	/// need none.
	const RECIPIENT: &str =
		"age1ql3z7hjy54pw3hyww5ayyfg7zqgvc7w3j2elw8zmrj2kg5sfn9aqmcac8p";

	#[crate::test]
	fn accepts_the_age_grammar() {
		AgeRecipient::new(RECIPIENT)
			.unwrap()
			.as_str()
			.xpect_eq(RECIPIENT);
		// uppercase bech32 is the same key, stored the way age prints it
		AgeRecipient::new(RECIPIENT.to_uppercase())
			.unwrap()
			.as_str()
			.xpect_eq(RECIPIENT);
		AgeRecipient::new(format!("  {RECIPIENT}\n"))
			.unwrap()
			.as_str()
			.xpect_eq(RECIPIENT);
	}

	#[crate::test]
	fn rejects_a_non_age1_string() {
		AgeRecipient::new("ssh-ed25519 AAAAC3Nza")
			.unwrap_err()
			.to_string()
			.xpect_contains("age-keygen");
		// a typo fails the checksum
		let mut typo = RECIPIENT.to_string();
		typo.replace_range(10..11, "x");
		AgeRecipient::new(&typo)
			.unwrap_err()
			.to_string()
			.xpect_contains("bech32");
		// the right encoding under the wrong prefix
		AgeRecipient::new("bc1qw508d6qejxtdg4y5r3zarvary0c5xw7kv8f3t4")
			.unwrap_err()
			.to_string()
			.xpect_contains("prefix");
	}

	#[cfg(feature = "json")]
	#[crate::test]
	fn serde_is_the_string() {
		let recipient = AgeRecipient::new(RECIPIENT).unwrap();
		let json = serde_json::to_string(&recipient).unwrap();
		json.xpect_eq(format!("\"{recipient}\""));
		serde_json::from_str::<AgeRecipient>(&json)
			.unwrap()
			.xpect_eq(recipient);
		serde_json::from_str::<AgeRecipient>("\"age1nope\"").unwrap_err();
	}

	#[crate::test]
	fn from_reflect_validates() {
		let recipient = AgeRecipient::new(RECIPIENT).unwrap();
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

#[cfg(all(test, feature = "secrets"))]
mod encrypt_test {
	use crate::prelude::*;

	const ARMOR_HEADER: &str = "-----BEGIN AGE ENCRYPTED FILE-----";

	/// The grammar validator agrees with the age crate on a generated key.
	#[crate::test]
	fn a_generated_recipient_validates() {
		let recipient = AgeIdentity::generate().to_recipient();
		AgeRecipient::new(recipient.as_str())
			.unwrap()
			.xpect_eq(recipient);
	}

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
	fn refuses_an_empty_list() {
		AgeRecipient::encrypt(&[], b"nobody")
			.unwrap_err()
			.to_string()
			.xpect_contains("age-keygen");
	}
}
