#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![deny(missing_docs)]

mod client;
mod tools;
mod types;
/// Re-export the typed header module at crate level.
pub use types::header;
/// Alias for [`header`] for ergonomic typed header access.
pub use types::headers;
/// Re-export media-type-driven serialization at crate level.
mod object_storage;
mod server;
/// WebSocket client and server implementations.
pub mod sockets;

/// Prelude module re-exporting commonly used items.
pub mod prelude {
	/// JavaScript analytics snippet for client-side tracking.
	#[cfg(feature = "json")]
	pub const ANALYTICS_JS: &str = include_str!("object_storage/analytics.js");
	/// Default port for a beet server: `8337` (BEET).
	pub const DEFAULT_SERVER_PORT: u16 = 8337;
	/// Default port for WebSocket connections.
	pub const DEFAULT_SOCKET_PORT: u16 = 8339;
	/// Default URL for local server connections.
	pub const DEFAULT_SERVER_LOCAL_URL: &str = "http://127.0.0.1:8337";
	/// Default port for the webdriver (chromedriver, geckodriver, etc.).
	pub const DEFAULT_WEBDRIVER_PORT: u16 = 8340;
	/// Default port for WebSocket connections in webdriver sessions.
	pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8341;

	pub use crate::client::*;
	pub use crate::object_storage::*;
	pub use crate::server::*;
	pub use crate::sockets;
	pub use crate::tools::*;
	pub use crate::types::*;
	pub use beet_core::prelude::*;

	// Re-export common types from dependencies
	pub use bevy::tasks::futures_lite::StreamExt;
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
