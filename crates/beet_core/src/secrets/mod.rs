//! The secrets document: a plaintext index of records plus one age blob per
//! group, sealed and opened with the vault layer (`crate::vault`).
//!
//! - [`SecretsDocument`]: the index of [`SecretRecord`]s and a sealed
//!   [`SecretsGroup`] per recipient list, opened into [`OpenSecrets`]
//! - [`Secrets`]: the `<Secrets path=".."/>` declaration, and the runner's
//!   convention for the same file

mod open_secrets;
mod secret_record;
mod secrets;
mod secrets_document;

pub use open_secrets::*;
pub use secret_record::*;
pub use secrets::*;
pub use secrets_document::*;
