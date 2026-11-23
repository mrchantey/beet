//! The secrets document over stores, its `<Secrets>` load and the `secrets`
//! verbs, on the document in `beet_core::secrets` and the vault layer in
//! `crate::vault`.
//!
//! - [`SecretsHandle`]: a document at a path in a store, resolved from a
//!   path or a store uri
//! - [`SecretsQuery`]: a document by `<Secrets>` label or by path
//! - the `<Secrets>` load on scene build, gating `Ready`
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`
//! - `aws`: one provider's authority over the account it deploys into, this
//!   machine's identity, a person's elevation and sign in, and the verbs
//!   reaching through them (native, `fs` and `aws` only)

mod actions;
// native and fs only: the verbs read this machine's files and run the cli
#[cfg(all(feature = "fs", feature = "aws", not(target_arch = "wasm32")))]
mod aws;
mod secrets_handle;
mod secrets_load;
mod secrets_plugin;
mod secrets_query;

pub use actions::*;
#[cfg(all(feature = "fs", feature = "aws", not(target_arch = "wasm32")))]
pub use aws::*;
pub use secrets_handle::*;
pub(crate) use secrets_load::*;
pub use secrets_plugin::*;
pub use secrets_query::*;
