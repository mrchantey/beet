//! The secrets system's primitives and its document: age identities and
//! recipients, the identity file, the passphrase envelope, and the typed
//! secrets document with a blob per group.
//!
//! Everything cryptographic goes through the `age` crate, never the cli, and
//! every ciphertext beet writes is a plain age file, so it opens with
//! `age -d -i ~/.config/beet/age/keys.txt <file>` on any laptop with no beet
//! in reach. The cli is the human's disaster-day escape hatch, not the
//! implementation.
//!
//! - [`AgeIdentity`]: the private key, `AGE-SECRET-KEY-1..`, one per human
//! - [`AgeRecipient`]: the public half, `age1..`, safe in a document and git
//! - [`AgeIdentityFile`]: the `keys.txt` grammar and where it is found
//! - [`AgePassphrase`]: the scrypt envelope for an identity's backup
//! - [`SecretsDocument`]: the index of [`SecretRecord`]s and a sealed
//!   [`SecretsGroup`] per recipient list, opened into [`OpenSecrets`]
//! - [`Secrets`]: the `<Secrets path=".."/>` declaration, and the runner's
//!   convention for the same file

mod age_identity;
mod age_identity_file;
mod age_passphrase;
mod age_recipient;
mod open_secrets;
mod secret_record;
mod secrets;
mod secrets_document;

pub use age_identity::*;
pub use age_identity_file::*;
pub use age_passphrase::*;
pub use age_recipient::*;
pub use open_secrets::*;
pub use secret_record::*;
pub use secrets::*;
pub use secrets_document::*;
