//! SSH client and server implementations.
//!
//! Provides cross-platform SSH support for Bevy applications.
//!
//! - **Server**: [`SshServer`] accepts incoming connections. Each connection
//!   spawns a child entity with [`SshPeerInfo`] and bidirectional
//!   [`SshDataSend`]/[`SshDataRecv`] event flow.
//! - **Client**: [`SshSession::insert_on_connect`] establishes an outbound session.
//!
//! ## Features
//!
//! - `russh_server` — enables [`SshServer`] on native targets
//! - `russh_client` — enables [`SshSession::insert_on_connect`] on native targets
mod ssh_data;
mod ssh_server;
mod ssh_session;

pub use ssh_data::*;
pub use ssh_server::*;
pub use ssh_session::*;

#[cfg(all(feature = "russh_client", not(target_arch = "wasm32")))]
pub(crate) mod impl_russh_client;
#[cfg(all(feature = "russh_server", not(target_arch = "wasm32")))]
pub(crate) mod impl_russh_server;
