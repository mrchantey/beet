//! Vault I/O over stores, the vault resolver and the `secrets` verbs, on the
//! primitives in `beet_core::secrets`. The registration rides `std` (the
//! declarations are data every build loads); the rest rides `secrets`.
//!
//! - [`VaultHandle`]: a vault resolved to a store, a path and its recipients,
//!   the read/write seam
//! - [`VaultQuery`]: `--vault=<label or path>` to a handle
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`

#[cfg(feature = "secrets")]
mod actions;
mod secrets_plugin;
#[cfg(feature = "secrets")]
mod vault_io;
#[cfg(feature = "secrets")]
mod vault_query;

#[cfg(feature = "secrets")]
pub use actions::*;
pub use secrets_plugin::*;
#[cfg(feature = "secrets")]
pub use vault_io::*;
#[cfg(feature = "secrets")]
pub use vault_query::*;
