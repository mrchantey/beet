#![doc = include_str!("../README.md")]
#![deny(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

beet_core::test_main!();

/// Transport-agnostic request/response wire types — the no_std core.
mod types;
/// Re-export the typed header module at crate level.
pub use types::header;
/// Alias for [`header`] for ergonomic typed header access.
pub use types::headers;
// `client` is no_std-capable: only `send.rs` (scheme routing, the data: URI
// path, and the `set_http_client` transport hook) compiles unconditionally —
// the concrete transports (reqwest/ureq/web-sys/file) stay feature-gated.
mod client;
// `store` is no_std-capable at its core (BlobStore, BlobStoreProvider,
// InMemoryStore); the concrete backends (fs/s3/dynamo/local-storage) and
// StorePlugin stay feature/std-gated inside the module.
mod store;
// std-only: transports, servers, sockets, and the action integration.
#[cfg(feature = "std")]
mod actions;
#[cfg(feature = "std")]
mod store_actions;
#[cfg(feature = "std")]
mod net_plugin;
#[cfg(feature = "std")]
mod server;
/// WebSocket client and server implementations.
#[cfg(feature = "std")]
pub mod sockets;
/// SSH client and server implementations.
#[cfg(any(feature = "russh_server", feature = "russh_client"))]
pub mod ssh;
/// WebDriver BiDi client for automated browser testing.
#[cfg(all(feature = "webdriver", not(target_arch = "wasm32")))]
pub mod webdriver;

/// Prelude module re-exporting commonly used items.
pub mod prelude {
	/// JavaScript analytics snippet for client-side tracking.
	pub const ANALYTICS_JS: &str = include_str!("store/analytics.js");
	/// Default port for a beet server: `8337` (BEET).
	pub const DEFAULT_SERVER_PORT: u16 = 8337;
	/// Default port for SSH connections: `8322`.
	pub const DEFAULT_SSH_PORT: u16 = 8322;
	/// Default port for WebSocket connections.
	pub const DEFAULT_SOCKET_PORT: u16 = 8339;
	/// Default URL for local server connections.
	pub const DEFAULT_SERVER_LOCAL_URL: &str = "http://127.0.0.1:8337";
	/// Default port for the webdriver (chromedriver, geckodriver, etc.).
	pub const DEFAULT_WEBDRIVER_PORT: u16 = 8340;
	/// Default port for WebSocket connections in webdriver sessions.
	pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8341;

	#[cfg(feature = "std")]
	pub use crate::actions::*;
	pub use crate::store::*;
	#[cfg(feature = "std")]
	pub use crate::store_actions::*;
	pub use crate::client::*;
	#[cfg(feature = "std")]
	pub use crate::net_plugin::*;
	#[cfg(feature = "std")]
	pub use crate::server::*;
	#[cfg(feature = "std")]
	pub use crate::sockets;
	#[cfg(any(feature = "russh_server", feature = "russh_client"))]
	pub use crate::ssh::*;
	pub use crate::types::*;
	#[cfg(all(feature = "webdriver", not(target_arch = "wasm32")))]
	pub use crate::webdriver;
	// Re-export core types used in beet_net's public API
	pub use beet_core::prelude::MediaBytes;
	pub use beet_core::prelude::MediaType;
	pub use beet_core::prelude::SmolPath;

	// Re-export common types from dependencies
	pub use bevy::tasks::futures_lite::StreamExt;
	#[cfg(feature = "std")]
	pub use uuid::Uuid;
}

/// Re-exports of dependency crates for downstream use.
pub mod exports {
	pub use bevy::tasks::futures_lite;
	#[cfg(feature = "http")]
	pub use eventsource_stream;
	#[cfg(feature = "http")]
	pub use http;
	#[cfg(all(feature = "hyper", not(target_arch = "wasm32")))]
	pub use http_body_util;
}
