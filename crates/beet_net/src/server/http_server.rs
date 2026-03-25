//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;
use std::future::Future;

/// HTTP server that listens for incoming requests, triggering a [`Tool::<Request,Response>`] call.
///
/// When spawned, this component automatically starts a server on the specified port.
/// The underlying implementation depends on compile-time feature flags:
/// - Default: Lightweight mini HTTP server using `async-io` TCP
/// - `hyper`: Full-featured Hyper HTTP server
/// - `lambda`: AWS Lambda runtime
///
/// # Example
///
/// ```ignore
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// world.spawn((
///     HttpServer::default(),
///     exchange_handler(|req| req.mirror()),
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
	/// The host address to bind to. Defaults to `[127, 0, 0, 1]` (localhost).
	/// Use `[0, 0, 0, 0]` to listen on all interfaces (required for deployed servers).
	pub host: [u8; 4],
}

impl Default for HttpServer {
	fn default() -> Self {
		Self {
			port: DEFAULT_SERVER_PORT,
			host: [127, 0, 0, 1],
		}
	}
}

impl HttpServer {
	/// Creates a new server configured to listen on the specified port.
	pub fn new(port: u16) -> Self {
		Self {
			port,
			..Default::default()
		}
	}
	/// Creates a new server configured to listen on all interfaces
	/// (i.e., host address `[0, 0, 0, 0]`) on the specified port.
	pub fn new_all_interfaces(port: u16) -> Self {
		Self {
			port,
			host: [0, 0, 0, 0],
		}
	}
	/// Sets the host address to bind to.
	pub fn with_host(mut self, host: [u8; 4]) -> Self {
		self.host = host;
		self
	}
}

// Using queue_async allows a ServerHandler to be inserted, instead of running
// immediately and using the one inserted via Required.
#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(test)]
	return;
	#[cfg(feature = "lambda")]
	world
		.commands()
		.entity(cx.entity)
		.queue_async(super::start_lambda_server);

	#[cfg(all(feature = "hyper", not(feature = "lambda")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async(super::start_hyper_server);

	#[cfg(all(not(feature = "hyper"), not(feature = "lambda")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async(super::start_mini_http_server);
}


impl HttpServer {
	/// Creates a new server with an auto-incrementing port for testing.
	///
	/// Each call returns a server on a different port, starting from
	/// [`DEFAULT_SERVER_TEST_PORT`], to avoid collisions in parallel tests.
	///
	/// We don't automatically assign server in tests so it must be provided.
	pub fn new_test<Func, Fut>(run_server: Func) -> (HttpServer, OnSpawn)
	where
		Func: 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Sync + Future<Output = Result>,
	{
		use std::sync::atomic::AtomicU16;
		use std::sync::atomic::Ordering;
		static PORT: AtomicU16 = AtomicU16::new(DEFAULT_SERVER_TEST_PORT);
		(
			Self {
				port: PORT.fetch_add(1, Ordering::SeqCst),
				..default()
			},
			OnSpawn::new_async(run_server),
		)
	}

	/// Returns the local URL for connecting to this server.
	pub fn local_url(&self) -> String {
		format!("http://127.0.0.1:{}", self.port)
	}
}


#[cfg(test)]
#[cfg(feature = "ureq")]
pub(crate) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Shared test suite runner for HTTP server implementations.
	///
	/// Spawns a server with a mirror exchange handler, sends requests,
	/// and verifies responses round-trip correctly.
	#[track_caller]
	pub async fn test_server<Func, Fut>(run_server: Func)
	where
		Func: 'static + Send + Sync + FnOnce(AsyncEntity) -> Fut,
		Fut: 'static + Send + Sync + Future<Output = Result>,
	{
		let server = HttpServer::new_test(run_server);
		let url = server.0.local_url();
		let _handle = std::thread::spawn(|| {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn_then((
					server,
					exchange_handler(move |req| {
						Response::ok().with_body(req.take().body)
					}),
				))
				.run();
		});
		time_ext::sleep_millis(100).await;

		// basic request-response roundtrip
		for _ in 0..3 {
			Request::post(&url)
				.send()
				.await
				.unwrap()
				.into_result()
				.await
				.xpect_ok();
		}

		// roundtrip with a text body
		let response = Request::post(&url)
			.with_body("hello")
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.unwrap();
		let body_text = response.text().await.unwrap();
		body_text.xpect_eq("hello");
	}
}
