//! The secrets system's primitives: age identities and recipients, the vault
//! declarations and their document formats, and the `.env.age` loader.
//!
//! Everything cryptographic goes through the `age` crate, never the cli, and
//! writes the same format, so every file beet produces opens with
//! `age -d -i ~/.config/beet/age/keys.txt <file>` on any laptop: the cli is
//! the human's disaster-day escape hatch, not the implementation.
//!
//! The declarations are data and ride `std` (the `.env` grammar and the
//! format names are no_std), so a lean binary loads a document naming a
//! `<Vault>` whole; the age machinery rides `secrets`:
//!
//! - [`AgeRecipient`]: the public half, `age1..`, safe in markup and git
//! - [`Vault`], [`AgeRecipients`]: the declarations, a file and who reads it
//! - [`VaultFormat`], [`EnvDocument`]: the formats, and the `.env` grammar
//!   `env_ext::parse_dotenv` shares
//! - [`AgeIdentity`]: the private key, `AGE-SECRET-KEY-1..`, one per human
//! - [`AgeIdentityFile`]: the `keys.txt` grammar and where it is found
//! - [`AgePassphrase`]: the scrypt envelope for an identity's backup
//! - [`VaultDocument`]: a vault's content in any format, with its metadata

#[cfg(feature = "secrets")]
mod age_identity;
#[cfg(feature = "secrets")]
mod age_identity_file;
#[cfg(feature = "secrets")]
mod age_passphrase;
#[cfg(feature = "std")]
mod age_recipient;
mod env_document;
#[cfg(feature = "std")]
mod vault;
#[cfg(feature = "secrets")]
mod vault_document;
mod vault_format;

#[cfg(feature = "secrets")]
pub use age_identity::*;
#[cfg(feature = "secrets")]
pub use age_identity_file::*;
#[cfg(feature = "secrets")]
pub use age_passphrase::*;
#[cfg(feature = "std")]
pub use age_recipient::*;
pub use env_document::*;
#[cfg(feature = "std")]
pub use vault::*;
#[cfg(feature = "secrets")]
pub use vault_document::*;
pub use vault_format::*;
