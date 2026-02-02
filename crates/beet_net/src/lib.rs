#![doc = include_str!("../README.md")]
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(beet_core::test_runner))]
#![cfg_attr(feature = "aws", feature(if_let_guard))]
#![feature(fn_traits, unboxed_closures)]
#![deny(missing_docs)]

mod client;
mod exchange;
mod object_storage;
mod server;
/// WebSocket client and server implementations.
pub mod sockets;

/// Prelude module re-exporting commonly used items.
pub mod prelude {
	/// JavaScript analytics snippet for client-side tracking.
	pub const ANALYTICS_JS: &str = include_str!("object_storage/analytics.js");
	/// Default port for a beet server: `8337` (BEET).
	pub const DEFAULT_SERVER_PORT: u16 = 8337;
	/// Default port for test servers.
	pub const DEFAULT_SERVER_TEST_PORT: u16 = 8400;
	/// Default port for WebSocket connections.
	pub const DEFAULT_SOCKET_PORT: u16 = 8339;
	/// Default port for test WebSocket connections.
	pub const DEFAULT_SOCKET_TEST_PORT: u16 = 8500;
	/// Default URL for local server connections.
	pub const DEFAULT_SERVER_LOCAL_URL: &str = "http://127.0.0.1:8337";
	/// Default port for the webdriver (chromedriver, geckodriver, etc.).
	pub const DEFAULT_WEBDRIVER_PORT: u16 = 8340;
	/// Default port for WebSocket connections in webdriver sessions.
	pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8341;

	pub use crate::client::*;
	pub use crate::exchange::*;
	pub use crate::object_storage::*;
	pub use crate::server::*;
	pub use crate::sockets;

	// Re-export common types from dependencies
	pub use http::header;
	pub use url::Url;
	pub use uuid::Uuid;

	pub use bevy::tasks::futures_lite::StreamExt;
}

/// Re-exports of dependency crates for downstream use.
pub mod exports {
	pub use bevy::tasks::futures_lite;
	pub use eventsource_stream;
	pub use http;
	pub use http_body_util;
	pub use url;
}
