//! Age files over stores and the `vault` verbs, on the primitives in
//! `beet_core::vault`.
//!
//! - [`VaultHandle`]: an age file at a path in a store, resolved from a path
//!   or a store uri
//! - the verbs, one file each under `actions`, mounted by `<VaultRoutes/>`
//!
//! The secrets document (`crate::secrets`) is the layer above: its groups
//! seal with the same primitives and its verbs live in their own namespace.

mod actions;
#[cfg(test)]
pub(crate) mod test_support;
mod vault_handle;
mod vault_plugin;

pub use actions::*;
pub use vault_handle::*;
pub use vault_plugin::*;
