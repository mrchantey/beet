//! The private half of an age key pair.

use crate::prelude::*;
use age::secrecy::ExposeSecret;
use core::fmt;
use core::str::FromStr;
use std::io::Read;

/// A person's age identity, `AGE-SECRET-KEY-1..`: the private key that opens
/// every vault naming its [`AgeRecipient`]. One per human, kept in the
/// [`AgeIdentityFile`] in the OS config; never in a vault, a cloud or on argv.
///
/// [`Debug`] redacts, so an identity can never reach a log by accident.
/// [`Display`] is the key itself, for the identity file writers and nothing
/// else. Two identities are equal when their recipients are.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let identity = AgeIdentity::generate();
/// let ciphertext =
/// 	AgeRecipient::encrypt(&[identity.to_recipient()], b"hello").unwrap();
/// identity
/// 	.decrypt(ciphertext.as_bytes())
/// 	.unwrap()
/// 	.xpect_eq(b"hello".to_vec());
/// ```
#[derive(Clone)]
pub struct AgeIdentity(age::x25519::Identity);

impl AgeIdentity {
	/// What every identity string starts with, how an inline
	/// [`AgeIdentityFile::ENV_VAR`] is told from a path.
	pub const PREFIX: &'static str = "AGE-SECRET-KEY-1";

	/// A fresh identity from the platform's entropy source.
	pub fn generate() -> Self { Self(age::x25519::Identity::generate()) }

	/// The public half, what a vault is encrypted to.
	pub fn to_recipient(&self) -> AgeRecipient {
		AgeRecipient::new_unchecked(self.0.to_public().to_string())
	}

	/// Decrypt an age file, armored or binary, that was encrypted to this
	/// identity's recipient. Errors when it was encrypted to someone else.
	pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
		Self::decrypt_any([self], ciphertext)
	}

	/// Decrypt an age file, armored or binary, with whichever of
	/// `identities` it names: an identity file tries every line it holds.
	/// Errors when none matches.
	pub fn decrypt_any<'a>(
		identities: impl IntoIterator<Item = &'a Self>,
		ciphertext: &[u8],
	) -> Result<Vec<u8>> {
		let identities = identities.into_iter().collect::<Vec<_>>();
		if identities.is_empty() {
			bevybail!("no identity to decrypt with");
		}
		let decryptor = age::Decryptor::new_buffered(
			age::armor::ArmoredReader::new(ciphertext),
		)?;
		if decryptor.is_scrypt() {
			bevybail!(
				"this age file is encrypted to a passphrase, not to an \
				identity: open it with `AgePassphrase` instead"
			);
		}
		let mut reader = match decryptor.decrypt(
			identities
				.iter()
				.map(|identity| &identity.0 as &dyn age::Identity),
		) {
			Err(age::DecryptError::NoMatchingKeys) => bevybail!(
				"none of the {} identities can open this age file: it was \
				encrypted to other recipients",
				identities.len()
			),
			other => other?,
		};
		let mut plaintext = Vec::new();
		reader.read_to_end(&mut plaintext)?;
		plaintext.xok()
	}
}

impl FromStr for AgeIdentity {
	type Err = BevyError;

	/// Parse an identity line. The input is never echoed: it may be most of a
	/// secret.
	fn from_str(value: &str) -> Result<Self> {
		value
			.trim()
			.parse::<age::x25519::Identity>()
			.map(Self)
			.map_err(|err| {
				bevyhow!(
					"not an age identity ({err}): an identity is one line \
					starting `{}`",
					Self::PREFIX
				)
			})
	}
}

/// The secret key line, for the identity file writers and nothing else.
impl fmt::Display for AgeIdentity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.0.to_string().expose_secret())
	}
}

/// Redacted: an identity never reaches a log.
impl fmt::Debug for AgeIdentity {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("AgeIdentity(<redacted>)")
	}
}

impl PartialEq for AgeIdentity {
	fn eq(&self, other: &Self) -> bool {
		self.to_recipient() == other.to_recipient()
	}
}
impl Eq for AgeIdentity {}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn roundtrips_through_its_string() {
		let identity = AgeIdentity::generate();
		let text = identity.to_string();
		text.starts_with(AgeIdentity::PREFIX).xpect_true();
		text.parse::<AgeIdentity>().unwrap().xpect_eq(identity);
	}

	#[crate::test]
	fn debug_redacts() {
		let identity = AgeIdentity::generate();
		format!("{identity:?}")
			.xpect_eq("AgeIdentity(<redacted>)")
			.xnot()
			.xpect_contains("AGE-SECRET-KEY");
	}

	#[crate::test]
	fn rejects_a_non_identity_without_echoing_it() {
		"definitely-not-a-key"
			.parse::<AgeIdentity>()
			.unwrap_err()
			.to_string()
			.xpect_contains(AgeIdentity::PREFIX)
			.xnot()
			.xpect_contains("definitely");
	}

	// the cli writes binary unless asked to armor, and beet reads both
	#[crate::test]
	fn decrypts_binary_as_well_as_armored() {
		let identity = AgeIdentity::generate();
		let recipient = identity
			.to_recipient()
			.as_str()
			.parse::<age::x25519::Recipient>()
			.unwrap();
		let binary = age::encrypt(&recipient, b"binary").unwrap();
		identity
			.decrypt(&binary)
			.unwrap()
			.xpect_eq(b"binary".to_vec());
	}

	// age's own errors render through its embedded message catalogue, which a
	// debug wasm build only has because `rust-embed/debug-embed` is on
	#[crate::test]
	fn names_a_non_age_file() {
		AgeIdentity::generate()
			.decrypt(
				"not an age file, but long enough to be a header\n"
					.repeat(4)
					.as_bytes(),
			)
			.unwrap_err()
			.to_string()
			.to_lowercase()
			.xpect_contains("header");
	}

	#[crate::test]
	fn refuses_a_passphrase_file() {
		let ciphertext = AgePassphrase::new("hunter2")
			.with_work_factor(10)
			.encrypt(b"hi")
			.unwrap();
		AgeIdentity::generate()
			.decrypt(ciphertext.as_bytes())
			.unwrap_err()
			.to_string()
			.xpect_contains("passphrase");
	}
}
