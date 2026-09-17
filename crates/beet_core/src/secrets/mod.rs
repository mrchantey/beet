//! age primitives: the identity a human holds, the recipient a vault is
//! encrypted to, the identity file in the OS config and the passphrase
//! envelope an identity is backed up in.
//!
//! Everything here goes through the `age` crate, never the cli, and writes the
//! same format, so every file beet produces opens with
//! `age -d -i ~/.config/beet/age/keys.txt <file>` on any laptop: the cli is
//! the human's disaster-day escape hatch, not the implementation.
//!
//! - [`AgeIdentity`]: the private key, `AGE-SECRET-KEY-1..`, one per human
//! - [`AgeRecipient`]: the public half, `age1..`, safe in markup and git
//! - [`AgeIdentityFile`]: the `keys.txt` grammar and where it is found
//! - [`AgePassphrase`]: the scrypt envelope for an identity's backup

mod age_identity;
mod age_identity_file;
mod age_passphrase;
mod age_recipient;

pub use age_identity::*;
pub use age_identity_file::*;
pub use age_passphrase::*;
pub use age_recipient::*;
