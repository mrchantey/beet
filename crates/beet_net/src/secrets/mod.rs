//! The secrets document over stores, its `<Secrets>` load and the `secrets`
//! verbs, on the primitives and the document in `beet_core::secrets`.
//!
//! - [`VaultHandle`]: an age file or a secrets document at a path in a
//!   store, the byte layer every secret rides on, resolved from a path or a
//!   store uri
//! - [`SecretsQuery`]: a document by `<Secrets>` label or by path
//! - the `<Secrets>` load on scene build, gating `Ready`
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`

mod actions;
mod secrets_load;
mod secrets_plugin;
mod secrets_query;
mod vault_io;

pub use actions::*;
pub(crate) use secrets_load::*;
pub use secrets_plugin::*;
pub use secrets_query::*;
pub use vault_io::*;
