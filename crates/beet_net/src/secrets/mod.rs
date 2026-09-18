//! Age files over stores and the `secrets` verbs, on the primitives in
//! `beet_core::secrets`.
//!
//! - [`VaultHandle`]: an age file at a path in a store, the byte layer every
//!   secret rides on, resolved from a path or a store uri
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`

mod actions;
mod secrets_plugin;
mod vault_io;

pub use actions::*;
pub use secrets_plugin::*;
pub use vault_io::*;
