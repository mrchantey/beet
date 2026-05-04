//! SSH client and server implementations.
//!
//! Provides cross-platform SSH support for Bevy applications:
//!
//! - **Server**: [`SshServer`] accepts incoming SSH connections, spawning an
//!   [`SshConnection`] child entity per client session.
//! - **Client**: [`SshSession::connect`] establishes an outbound SSH session.
//!
//! ## Features
//!
//! - `russh_server` — enables [`SshServer`] on native targets
//! - `russh_client` — enables [`SshSession::connect`] on native targets
//!
//! ## Example
//!
//! ```ignore
//! # use beet_net::ssh::*;
//! # use beet_core::prelude::*;
//! # async fn run() -> Result<()> {
//! let session = SshSession::connect("127.0.0.1:2222", "user", "pass").await?;
//! # Ok(())
//! # }
//! ```
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
