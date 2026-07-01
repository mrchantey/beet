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
// The action/exchange integration (`exchange_handler`, …) only needs
// `beet_action`, so it rides the no_std-capable `action` feature, not `std`.
#[cfg(feature = "action")]
mod actions;
#[cfg(feature = "std")]
mod net_plugin;
#[cfg(feature = "std")]
mod store_actions;
// The server module is no_std-capable: only the `HttpServer` component and its
// `set_http_server` install hook compile unconditionally — the concrete
// backends (mini/hyper/lambda) and the cli/repl servers stay std/feature-gated
// inside the module.
mod server;
// The udp module is no_std-capable: the trait-only `UdpEndpoint`/`UdpSocket`
// seam compiles unconditionally; the std `async-io` impl is std-gated.
mod udp;
// mDNS rides the udp seam but is a distinct protocol, so it lives in its own
// module behind the `mdns` feature. The wire codec and browser engine are
// no_std; only the std socket driver `run_mdns_browser` needs `udp` + `std`.
#[cfg(feature = "mdns")]
mod mdns;
/// WebSocket client and server implementations.
#[cfg(feature = "sockets")]
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
	pub const ANALYTICS_JS: &str = include_str!("store/analytics/analytics.js");
	/// Default URL for local server connections.
	pub const DEFAULT_HTTP_LOCAL_URL: &str = "http://127.0.0.1:8337";
	/// Default port for a beet server.
	pub const DEFAULT_HTTP_PORT: u16 = 8337;
	/// Default port for WebSocket connections.
	pub const DEFAULT_SOCKET_PORT: u16 = 8338;
	/// Default port for SSH connections.
	pub const DEFAULT_SSH_PORT: u16 = 8339;
	/// Default port for the webdriver (chromedriver, geckodriver, etc.).
	pub const DEFAULT_WEBDRIVER_PORT: u16 = 8340;
	/// Default port for WebSocket connections in webdriver sessions: `8341`.
	pub const DEFAULT_WEBDRIVER_SESSION_PORT: u16 = 8341;

	/// Resolve the port a beet server listens on: the `explicit` value if set,
	/// else the `BEET_HTTP_PORT` environment variable, else [`DEFAULT_HTTP_PORT`].
	///
	/// The single source of truth for "which port", shared by [`HttpServer`] and
	/// the deploy blocks (`LightsailBlock`, `CloudflareContainerBlock`) so a markup
	/// port, an env override and the static default all resolve the same way.
	pub fn resolve_server_port(explicit: Option<u16>) -> u16 {
		explicit
			.or_else(|| {
				beet_core::prelude::env_ext::var("BEET_HTTP_PORT")
					.ok()
					.and_then(|val| val.parse().ok())
			})
			.unwrap_or(DEFAULT_HTTP_PORT)
	}

	#[cfg(feature = "action")]
	pub use crate::actions::*;
	pub use crate::client::*;
	#[cfg(feature = "mdns")]
	pub use crate::mdns::*;
	#[cfg(feature = "std")]
	pub use crate::net_plugin::*;
	pub use crate::server::*;
	#[cfg(feature = "sockets")]
	pub use crate::sockets;
	#[cfg(any(feature = "russh_server", feature = "russh_client"))]
	pub use crate::ssh::*;
	pub use crate::store::*;
	#[cfg(feature = "std")]
	pub use crate::store_actions::*;
	pub use crate::types::*;
	pub use crate::udp::*;
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
