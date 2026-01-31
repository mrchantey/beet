//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;

/// HTTP server that listens for incoming requests and routes them to handlers.
///
/// When spawned, this component automatically starts a server on the specified port.
/// The underlying implementation depends on compile-time feature flags:
/// - `lambda`: Uses AWS Lambda runtime
/// - Default: Uses Hyper HTTP server
///
/// # Example
///
/// ```ignore
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// world.spawn((
///     HttpServer::default(),
///     HandlerExchange::new(|req| req.mirror()),
/// ));
/// ```
#[derive(Clone, Component)]
#[component(on_add=on_add)]
#[require(ExchangeStats)]
pub struct HttpServer {
	/// The port the server listens on. This may be updated at runtime,
	/// for instance if the provided port is `0` it may be updated to
	/// some random available port by the os like `98304`.
	/// This is ignored by lambda_server
	pub port: u16,
}

impl Default for HttpServer {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
		}
	}
}

impl HttpServer {
	/// Creates a new server configured to listen on the specified port.
	pub fn new(port: u16) -> Self { Self { port } }
}

// using commands allows a ServerHandler to be inserted, instead of running immediately
// and using the one inserted via Required.
#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(feature = "lambda")]
	world
		.commands()
		.run_system_cached_with(super::start_lambda_server, cx.entity);

	#[cfg(not(feature = "lambda"))]
	world
		.commands()
		.run_system_cached_with(super::start_hyper_server, cx.entity);
}


impl HttpServer {
	/// Creates a new server with an auto-incrementing port for testing.
	///
	/// Each call returns a server on a different port, starting from
	/// [`DEFAULT_SERVER_TEST_PORT`], to avoid collisions in parallel tests.
	pub fn new_test() -> Self {
		use std::sync::atomic::AtomicU16;
		use std::sync::atomic::Ordering;
		static PORT: AtomicU16 = AtomicU16::new(DEFAULT_SERVER_TEST_PORT);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			..default()
		}
	}

	/// Returns the local URL for connecting to this server.
	pub fn local_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.port)
	}
}
