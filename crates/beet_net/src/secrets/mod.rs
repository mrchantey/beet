//! The secrets document over stores, its `<Secrets>` load and the `secrets`
//! verbs, on the document in `beet_core::secrets` and the vault layer in
//! `crate::vault`.
//!
//! - [`SecretsHandle`]: a document at a path in a store, resolved from a
//!   path or a store uri
//! - [`SecretsQuery`]: a document by `<Secrets>` label or by path
//! - the `<Secrets>` load on scene build, gating `Ready`
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`

mod actions;
mod secrets_handle;
mod secrets_load;
mod secrets_plugin;
mod secrets_query;

pub use actions::*;
pub use secrets_handle::*;
pub(crate) use secrets_load::*;
pub use secrets_plugin::*;
pub use secrets_query::*;
