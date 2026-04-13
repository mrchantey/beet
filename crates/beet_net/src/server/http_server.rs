//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;
use std::future::Future;

/// HTTP server that listens for incoming requests, triggering an [`Action::<Request,Response>`] call.
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
	/// The port the server listens on. `None` means the OS will assign
	/// an available port (equivalent to binding to port `0`).
	/// This is ignored by lambda_server.
	pub port: Option<u16>,
	/// The host address to bind to. Defaults to `[127, 0, 0, 1]` (localhost).
	/// Use `[0, 0, 0, 0]` to listen on all interfaces (required for deployed servers).
	pub host: [u8; 4],
}

impl Default for HttpServer {
	fn default() -> Self {
		Self {
			port: Some(DEFAULT_SERVER_PORT),
			host: [127, 0, 0, 1],
		}
	}
}

impl HttpServer {
	/// Creates a new server configured to listen on the specified port.
	pub fn new(port: u16) -> Self {
		Self {
			port: Some(port),
			..Default::default()
		}
	}
	/// Creates a new server configured to listen on all interfaces
	/// (i.e., host address `[0, 0, 0, 0]`) on the specified port.
	pub fn new_all_interfaces(port: u16) -> Self {
		Self {
			port: Some(port),
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
		.queue_async_local(super::start_lambda_server);

	#[cfg(all(feature = "hyper", not(feature = "lambda")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async_local(super::start_hyper_server);

	#[cfg(all(not(feature = "hyper"), not(feature = "lambda")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async_local(super::start_mini_http_server);
}


impl HttpServer {
	/// Creates a test server bound to an OS-assigned port.
	///
	/// Binds to port `0` so the OS picks a free port, avoiding
	/// collisions in parallel tests. The listener is kept alive and
	/// passed directly to the server function, eliminating port race conditions.
	///
	/// The `on_add` hook is disabled in tests, so the returned
	/// [`OnSpawn`] must be included in the spawn bundle to start
	/// the listener.
	pub fn new_test<Func, Fut>(run_server: Func) -> (HttpServer, OnSpawn)
	where
		Func: 'static
			+ Send
			+ Sync
			+ FnOnce(AsyncEntity, async_io::Async<std::net::TcpListener>) -> Fut,
		Fut: 'static + Send + Sync + Future<Output = Result>,
	{
		let listener = std::net::TcpListener::bind("127.0.0.1:0")
			.expect("failed to bind test server");
		let port = listener.local_addr().unwrap().port();
		let listener = async_io::Async::new(listener)
			.expect("failed to create async listener");
		(
			Self {
				port: Some(port),
				..default()
			},
			OnSpawn::new_async(move |entity| run_server(entity, listener)),
		)
	}

	/// Returns the local URL for connecting to this server.
	pub fn local_url(&self) -> String {
		let port = self.port.unwrap_or(0);
		format!("http://127.0.0.1:{}", port)
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
	pub async fn test_server<Func, Fut>(run_server: Func)
	where
		Func: 'static
			+ Send
			+ Sync
			+ FnOnce(AsyncEntity, async_io::Async<std::net::TcpListener>) -> Fut,
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
