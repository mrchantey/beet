//! ## Cross-platform networking utilities
//!
//! Beet net provides general purpose primitives for communicating between systems.
//!
#![cfg_attr(test, feature(test, custom_test_frameworks))]
#![cfg_attr(test, test_runner(sweet::test_runner))]
#![cfg_attr(feature = "aws", feature(if_let_guard))]


mod exchange;
mod object_storage;
mod server;
pub mod sockets;
mod transport;

pub mod prelude {

	pub const ANALYTICS_JS: &str = include_str!("object_storage/analytics.js");
	/// Default port for a beet server: `8337` (BEET)
	pub const DEFAULT_SERVER_PORT: u16 = 8337;
	pub const DEFAULT_SERVER_TEST_PORT: u16 = 8400;
	pub const DEFAULT_SOCKET_PORT: u16 = 8339;
	pub const DEFAULT_SOCKET_TEST_PORT: u16 = 8500;
	pub const DEFAULT_SERVER_LOCAL_URL: &str = "http://127.0.0.1:8337";
	/// Default port for the webdriver (chromedriver, geckodriver etc): 8338
	pub const DEFAULT_WEBDRIVER_PORT: u16 = 8340;
	/// Default port for websocket connections (geckodriver only, chromedriver uses default port): 8339
	pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8341;

	pub use crate::exchange::*;
	pub use crate::object_storage::*;
	pub use crate::server::*;
	pub use crate::sockets;
	// pub use crate::transport::*;

	// reexport common types
	pub use http::StatusCode;
	pub use http::header;
	pub use url::Url;
	pub use uuid::Uuid;

	pub use bevy::tasks::futures_lite::StreamExt;
}

pub mod exports {
	pub use bevy::tasks::futures_lite;
	pub use eventsource_stream;
	pub use http;
	pub use http_body_util;
	pub use url;
}
