//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::OnceLock;

/// Boxed server-start function: the [`ServerBackend::start`] shape, also used by
/// the [`ServerBackends`] registry and the runtime [`set_http_server`] seam.
///
/// This is the no_std-friendly server hook, mirroring [`HttpSendFn`] on the
/// client side. When no server backend feature (`server`/`hyper`/`lambda`) is
/// compiled in, [`HttpServer::start`] falls through to a function installed via
/// [`set_http_server`] — letting a downstream adapter (an embassy / esp WiFi
/// crate, …) plug in its own listener without living in `beet_net`.
///
/// It is handed an [`AsyncEntity`] for the spawned server (run on the async
/// layer, exactly like the built-in [`start_hyper_server`] /
/// `start_mini_http_server` backends) and returns a boxed future. The adapter
/// reads the [`HttpServer`] config off the entity, opens its own listener, and
/// dispatches each request back through `entity.exchange(req)`.
pub type HttpServerFn =
	fn(AsyncEntity) -> MaybeSendBoxedFuture<'static, Result>;

static HTTP_SERVER: OnceLock<HttpServerFn> = OnceLock::new();

/// Install the server backend used by [`HttpServer`] when no server feature is
/// compiled in. Call once at startup from the adapter crate; returns an error
/// if a backend has already been installed.
pub fn set_http_server(server: HttpServerFn) -> Result<()> {
	HTTP_SERVER
		.set(server)
		.map_err(|_| bevyhow!("HTTP server already installed"))
}

/// HTTP server that listens for incoming requests, triggering an [`Action::<Request,Response>`] call.
///
/// A long-running [`ServerBackend`]: spawning it pulls in the [`Server`]
/// orchestrator (via `#[require(Server)]`), which starts it through
/// [`HttpServer::start`]. The concrete listener depends on compile-time feature
/// flags:
/// - Default (`server`): Lightweight mini HTTP server using `async-io` TCP
/// - `hyper`: Full-featured Hyper HTTP server
/// - `lambda`: AWS Lambda runtime
/// - none of the above (eg `no_std` embedded): a backend installed at runtime
///   via [`set_http_server`]
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
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[require(Server)]
#[cfg_attr(feature = "std", require(ExchangeStats))]
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
		cfg_if! {
			if #[cfg(feature = "std")] {
				let port = env_ext::var("BEET_PORT")
					.ok()
					.and_then(|val| val.parse().ok())
					.unwrap_or(DEFAULT_SERVER_PORT);
				let host = env_ext::var("BEET_HOST")
					.ok()
					.map(|val| {
						if val == "0.0.0.0" {
							[0, 0, 0, 0]
						} else {
							[127, 0, 0, 1]
						}
					})
					.unwrap_or([127, 0, 0, 1]);
				Self {
					port: Some(port),
					host,
				}
			} else {
				// no_std: no environment to read, use the static defaults.
				Self {
					port: Some(DEFAULT_SERVER_PORT),
					host: [127, 0, 0, 1],
				}
			}
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

	/// Returns the local URL for connecting to this server.
	pub fn local_url(&self) -> String {
		let port = self.port.unwrap_or(0);
		format!("http://127.0.0.1:{}", port)
	}

	/// The socket address to bind, resolving port and host by precedence
	/// `--port` / `--host` argv param > component field > default (`0` =
	/// OS-assigned, localhost). Backends call this so a `--port=8080` overrides
	/// a declared `HttpServer { port }`.
	#[cfg(feature = "std")]
	pub fn socket_addr(&self) -> core::net::SocketAddr {
		let params = CliArgs::parse_env().params;
		let port = resolve_config(
			params.get("port").and_then(|val| val.parse().ok()),
			self.port,
			0,
		);
		let host = resolve_config(
			params.get("host").map(|val| {
				if val == "0.0.0.0" { [0, 0, 0, 0] } else { [127, 0, 0, 1] }
			}),
			Some(self.host),
			[127, 0, 0, 1],
		);
		(host, port).into()
	}
}

/// Marker [`HttpServer::start`] inserts in test builds instead of starting a
/// backend, proving the orchestrator reached it (including through reflect
/// inserts and the `#[require(Server)]` boot).
#[cfg(test)]
#[derive(Component)]
pub(crate) struct ServerHookFired;

impl ServerBackend for HttpServer {
	/// Start the compile-time-selected HTTP backend. In test builds this inserts
	/// [`ServerHookFired`] instead, proving the [`Server`] orchestrator reached
	/// the backend. With no backend feature compiled in (eg a no_std embedded
	/// target) it defers to a listener installed via [`set_http_server`].
	#[allow(unused_variables)]
	fn start(entity: AsyncEntity) -> MaybeSendBoxedFuture<'static, Result> {
		cfg_if! {
			if #[cfg(test)] {
				Box::pin(async move {
					entity
						.with(|mut entity| { entity.insert(ServerHookFired); })
						.await
				})
			} else if #[cfg(all(feature = "lambda", not(target_arch = "wasm32")))] {
				Box::pin(super::start_lambda_server(entity))
			} else if #[cfg(all(feature = "hyper", not(target_arch = "wasm32")))] {
				Box::pin(super::start_hyper_server(entity))
			} else if #[cfg(all(feature = "server", not(target_arch = "wasm32")))] {
				Box::pin(super::start_mini_http_server(entity))
			} else {
				// No backend compiled in: defer to a listener installed at runtime
				// via `set_http_server`, reading the `HttpServer` config off the
				// entity and routing requests back through `entity.exchange(req)`.
				match HTTP_SERVER.get() {
					Some(start) => start(entity),
					None => Box::pin(async {
						bevybail!(
							"No HTTP server backend configured. Enable a server \
							 feature (server/hyper/lambda) or install one via \
							 set_http_server(...)."
						)
					}),
				}
			}
		}
	}
}


/// std-only constructors and the on-hardware integration test suite.
#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
mod std_impl {
	use super::*;
	use std::future::Future;

	impl HttpServer {
		/// Creates a test server bound to an OS-assigned port.
		///
		/// Binds to port `0` so the OS picks a free port, avoiding
		/// collisions in parallel tests. The listener is kept alive and
		/// passed directly to the server function, eliminating port race conditions.
		///
		/// The returned [`OnSpawn`] runs the real listener; include it in the
		/// spawn bundle. In `beet_net`'s own unit tests the [`ServerBackend`]
		/// start stub only inserts [`ServerHookFired`], so the listener comes
		/// from this `OnSpawn`.
		pub fn new_test<Func, Fut>(run_server: Func) -> (HttpServer, OnSpawn)
		where
			Func: 'static
				+ Send
				+ Sync
				+ FnOnce(
					AsyncEntity,
					async_io::Async<std::net::TcpListener>,
				) -> Fut,
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
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::reflect::ReflectComponent;

	/// A reflect insert (the BSX spread path, eg `{(HttpServer{port:8080})}`)
	/// brings the [`Server`] orchestrator in via `#[require(Server)]` and boots
	/// the backend exactly like a regular spawn. `beet_net`'s own unit-test
	/// backend stub inserts [`ServerHookFired`] instead of binding a port.
	#[beet_core::test]
	async fn boots_on_reflect_insert() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn_empty().id();
		let registry = app.world().resource::<AppTypeRegistry>().clone();
		// reflect-insert `HttpServer`, the same path a BSX `{(HttpServer{..})}`
		// spread takes; the `#[require(Server)]` fires through it.
		{
			let registry = registry.read();
			registry
				.get(core::any::TypeId::of::<HttpServer>())
				.unwrap()
				.data::<ReflectComponent>()
				.unwrap()
				.insert(
					&mut app.world_mut().entity_mut(entity),
					&HttpServer::new(8080),
					&registry,
				);
		}
		// the require brings `Server` in synchronously
		app.world().entity(entity).contains::<Server>().xpect_true();
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(8080));
		// settle the orchestrator's queued boot, which reaches the backend stub
		app.update_async().await;
		app.world()
			.entity(entity)
			.contains::<ServerHookFired>()
			.xpect_true();
	}

	/// With no `--port` argv param the component field drives the bind address
	/// (the middle tier of the `param > field > default` precedence).
	#[beet_core::test]
	fn socket_addr_uses_component_port() {
		HttpServer::new(8080).socket_addr().port().xpect_eq(8080);
	}
}

// needs `new_test` + `async_io` (server, native) and the ureq client.
#[cfg(test)]
#[cfg(all(feature = "ureq", feature = "server", not(target_arch = "wasm32")))]
pub(crate) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use std::future::Future;

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
				.spawn((
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
