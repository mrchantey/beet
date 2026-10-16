//! The secrets document over stores, its `<Secrets>` load and the `secrets`
//! verbs, on the document in `beet_core::secrets` and the vault layer in
//! `crate::vault`.
//!
//! - [`SecretsHandle`]: a document at a path in a store, resolved from a
//!   path or a store uri
//! - [`SecretsQuery`]: a document by `<Secrets>` label or by path
//! - the `<Secrets>` load on scene build, gating `Ready`
//! - the verbs, one file each under `actions`, mounted by `<SecretsRoutes/>`
//! - [`AgentIdentity`]: the global document holding this machine's own aws
//!   pair, and [`AdminSession`], the tmpfs session that elevates it
//! - [`MemberAccount`]: a repo's own account in the operator's Organization,
//!   and the roles that reach it from the management account

mod actions;
// native and fs only, like the `aws`/`admin` verbs that read it
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod agent_identity;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
mod member_account;
mod secrets_handle;
mod secrets_load;
mod secrets_plugin;
mod secrets_query;

pub use actions::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use agent_identity::*;
#[cfg(all(feature = "fs", not(target_arch = "wasm32")))]
pub use member_account::*;
pub use secrets_handle::*;
pub(crate) use secrets_load::*;
pub use secrets_plugin::*;
pub use secrets_query::*;
