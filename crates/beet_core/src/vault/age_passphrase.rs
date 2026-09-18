//! The passphrase envelope an identity is backed up in.

use crate::prelude::*;
use age::secrecy::SecretString;
use core::fmt;

/// A passphrase, the scrypt envelope for the one file that has no recipient
/// to be encrypted to: an identity's own backup, on a USB stick or paper.
/// Anyone with the passphrase reads the file, so it is for a human to hold
/// and nothing programmatic; a vault is encrypted to [`AgeRecipient`]s.
///
/// [`Debug`] redacts.
///
/// ## Example
///
/// ```
/// # use beet_core::prelude::*;
/// let passphrase = AgePassphrase::new("correct horse battery staple")
/// 	// the default costs about a second, a test wants less
/// 	.with_work_factor(10);
/// let ciphertext = passphrase.encrypt(b"AGE-SECRET-KEY-1...").unwrap();
/// passphrase
/// 	.decrypt(ciphertext.as_bytes())
/// 	.unwrap()
/// 	.xpect_eq(b"AGE-SECRET-KEY-1...".to_vec());
/// ```
pub struct AgePassphrase {
	passphrase: SecretString,
	/// The scrypt work factor `N = 2^log_n`, age's own choice when unset.
	work_factor: Option<u8>,
}

impl AgePassphrase {
	/// A passphrase a human typed.
	pub fn new(passphrase: impl Into<String>) -> Self {
		Self {
			passphrase: SecretString::from(passphrase.into()),
			work_factor: None,
		}
	}

	/// Override the scrypt work factor `N = 2^log_n`, `1..64`. Unset, age
	/// picks one costing about a second on this machine, which is what a
	/// backup wants; a test wants a small one.
	pub fn with_work_factor(mut self, log_n: u8) -> Self {
		self.work_factor = Some(log_n);
		self
	}

	/// Encrypt `plaintext` to this passphrase, as armored text.
	pub fn encrypt(&self, plaintext: &[u8]) -> Result<String> {
		let mut recipient =
			age::scrypt::Recipient::new(self.passphrase.clone());
		if let Some(log_n) = self.work_factor {
			if !(1..64).contains(&log_n) {
				bevybail!("scrypt work factor {log_n} is outside 1..64");
			}
			recipient.set_work_factor(log_n);
		}
		age::encrypt_and_armor(&recipient, plaintext)?.xok()
	}

	/// Decrypt a passphrase-encrypted age file, armored or binary. Errors when
	/// the passphrase is wrong or the file was encrypted to recipients
	/// instead.
	pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>> {
		let identity = age::scrypt::Identity::new(self.passphrase.clone());
		match age::decrypt(&identity, ciphertext) {
			Err(age::DecryptError::DecryptionFailed) => {
				bevybail!("the passphrase does not open this age file")
			}
			Err(age::DecryptError::NoMatchingKeys) => bevybail!(
				"this age file is encrypted to recipients, not to a \
				passphrase: open it with an identity instead"
			),
			other => other?.xok(),
		}
	}
}

/// Redacted: a passphrase never reaches a log.
impl fmt::Debug for AgePassphrase {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("AgePassphrase(<redacted>)")
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	#[crate::test]
	fn roundtrip() {
		let passphrase = AgePassphrase::new("hunter2").with_work_factor(10);
		let ciphertext = passphrase.encrypt(b"the identity").unwrap();
		ciphertext
			.as_str()
			.xpect_starts_with("-----BEGIN AGE ENCRYPTED FILE-----");
		passphrase
			.decrypt(ciphertext.as_bytes())
			.unwrap()
			.xpect_eq(b"the identity".to_vec());
	}

	#[crate::test]
	fn wrong_passphrase_fails() {
		let ciphertext = AgePassphrase::new("hunter2")
			.with_work_factor(10)
			.encrypt(b"the identity")
			.unwrap();
		AgePassphrase::new("hunter3")
			.decrypt(ciphertext.as_bytes())
			.unwrap_err()
			.to_string()
			.xpect_contains("passphrase");
	}

	#[crate::test]
	fn refuses_a_recipient_file() {
		let identity = AgeIdentity::generate();
		let ciphertext =
			AgeRecipient::encrypt(&[identity.to_recipient()], b"hi").unwrap();
		AgePassphrase::new("hunter2")
			.decrypt(ciphertext.as_bytes())
			.unwrap_err()
			.to_string()
			.xpect_contains("recipients");
	}

	#[crate::test]
	fn debug_redacts() {
		format!("{:?}", AgePassphrase::new("hunter2"))
			.xpect_eq("AgePassphrase(<redacted>)");
	}
}
