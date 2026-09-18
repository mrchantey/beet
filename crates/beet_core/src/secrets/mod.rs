//! The secrets document: a plaintext index of records plus one age blob per
//! group, sealed and opened with the vault layer (`crate::vault`).
//!
//! - [`SecretsDocument`]: the index of [`SecretRecord`]s and a sealed
//!   [`SecretsGroup`] per recipient list, opened into [`OpenSecrets`]
//! - [`Rotation`]: how a record's secret is rotated, declared at mint; plain
//!   data in every build, since a stack declaration renders it
//! - [`Secrets`]: the `<Secrets path=".."/>` declaration, and the runner's
//!   convention for the same file

#[cfg(feature = "vault")]
mod open_secrets;
mod rotation;
#[cfg(feature = "vault")]
mod secret_record;
#[cfg(feature = "vault")]
mod secrets;
#[cfg(feature = "vault")]
mod secrets_document;

#[cfg(feature = "vault")]
pub use open_secrets::*;
pub use rotation::*;
#[cfg(feature = "vault")]
pub use secret_record::*;
#[cfg(feature = "vault")]
pub use secrets::*;
#[cfg(feature = "vault")]
pub use secrets_document::*;
