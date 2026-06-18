//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::OnceLock;

/// Boxed server-start function: the no_std-friendly server hook, mirroring
/// [`HttpSendFn`] on the client side.
///
/// [`ServerPlugin`] installs one of the built-in backends (mini / hyper / lambda)
/// via [`set_http_server`] based on compile-time features; a downstream adapter
/// (an embassy / esp WiFi crate, …) installs its own without living in
/// [`beet_net`]. [`HttpServer`]'s start observer invokes the installed function.
///
/// It is handed an [`AsyncEntity`] for the spawned server and returns a boxed
/// future. The backend reads the [`HttpServer`] config off the entity, opens its
/// own listener, and dispatches each request back through `entity.exchange(req)`.
///
/// The future is a [`LocalBoxedFuture`] (never `Send`): the start observer always
/// drives it with `queue_async_local`, so it stays on the thread it was created
/// on. This lets a backend hold a thread-bound resource across an await — eg the
/// lambda backend's tokio runtime [`EnterGuard`](tokio::runtime::EnterGuard).
pub type HttpServerFn = fn(AsyncEntity) -> LocalBoxedFuture<'static, Result>;

static HTTP_SERVER: OnceLock<HttpServerFn> = OnceLock::new();

/// Install the backend [`HttpServer`] invokes on start. [`ServerPlugin`] calls
/// this for the compile-time-selected feature backend; a no_std adapter with no
/// compiled-in backend installs its own. Returns an error if one is already set.
pub fn set_http_server(server: HttpServerFn) -> Result<()> {
	HTTP_SERVER
		.set(server)
		.map_err(|_| bevyhow!("HTTP server already installed"))
}

/// The installed backend, if any.
pub fn http_server() -> Option<HttpServerFn> { HTTP_SERVER.get().copied() }

/// HTTP server that listens for incoming requests, triggering an
/// [`Action::<Request,Response>`] call.
///
/// A long-running server: a [`StartServer`] event whose filter passes `"http"`
/// boots it through the backend [`ServerPlugin`] installed via
/// [`set_http_server`], reading `--port` / `--host` from the event's `params`.
/// Booting inserts [`KeepAlive`] so the process persists. A [`StopServer`] event
/// (`"http"`) tears it down. A markup-spawned `<Router {(HttpServer{port:0})}>`
/// boots exactly the same way.
///
/// The concrete backend depends on compile-time features:
/// - Default (`server`): lightweight mini HTTP server using `async-io` TCP
/// - `hyper`: full-featured Hyper HTTP server
/// - `lambda`: AWS Lambda runtime
/// - none of the above (eg no_std embedded): a backend installed at runtime via
///   [`set_http_server`]
///
/// # Example
///
/// ```ignore
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// let host = world.spawn((
///     HttpServer::default(),
///     exchange_handler(|req| req.mirror()),
/// )).id();
/// world.entity_mut(host).trigger(StartServer::all);
/// ```
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
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
		// `env_ext::var` returns "not found" on no_std, so this reads `BEET_PORT`
		// / `BEET_HOST` where there is an environment and falls back to the static
		// defaults everywhere else, no feature gate needed.
		let port = env_ext::var("BEET_PORT")
			.ok()
			.and_then(|val| val.parse().ok())
			.unwrap_or(DEFAULT_SERVER_PORT);
		let host = env_ext::var("BEET_HOST")
			.ok()
			.map(|val| if val == "0.0.0.0" { [0, 0, 0, 0] } else { [127, 0, 0, 1] })
			.unwrap_or([127, 0, 0, 1]);
		Self {
			port: Some(port),
			host,
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

	/// The socket address to bind, from the component fields (`0` = OS-assigned,
	/// localhost the default host). The start observer applies any `--port` /
	/// `--host` from the [`StartServer`] event onto these fields before the
	/// backend reads them, so a `--port=8080` overrides a declared `port`.
	pub fn socket_addr(&self) -> core::net::SocketAddr {
		(self.host, self.port.unwrap_or(0)).into()
	}

	/// Overlays `--port` / `--host` from a [`StartServer`]'s params onto a copy of
	/// these fields, the resolved bind config the backend then reads.
	fn with_params(mut self, params: &MultiMap<SmolStr, SmolStr>) -> Self {
		if let Some(port) = params.get("port").and_then(|val| val.parse().ok()) {
			self.port = Some(port);
		}
		if let Some(host) = params.get("host") {
			self.host = if host == "0.0.0.0" {
				[0, 0, 0, 0]
			} else {
				[127, 0, 0, 1]
			};
		}
		self
	}
}

/// Registers the [`StartServer`] / [`StopServer`] observers on the host, so the
/// server boots when a start event whose filter passes `"http"` lands on it.
/// no_std, like the start/stop dispatch it registers: the async runtime
/// (`queue_async_local`) and the installed backend hook both build without std.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.observe_any(on_start_server)
		.observe_any(on_stop_server);
}

/// Shutdown signal for a running [`HttpServer`]: [`on_start_server`] stores the
/// sender on the host, [`start_http_server`] awaits the receiver alongside the
/// backend, and [`on_stop_server`] signals it to end the accept loop and drop the
/// listener (closing the port). A no_std one-shot channel, so an embedded backend
/// tears down the same way. Removed on stop, so a reboot installs a fresh one.
#[derive(Component)]
struct HttpServerShutdown(Option<OnceValue<()>>);

/// Boots the HTTP backend when a [`StartServer`] event passing `"http"` lands.
/// Applies the event's `--port` / `--host` onto the component, takes a [`KeepAlive`]
/// ref (a long-running server keeps the process up), then queues the installed
/// [`HttpServerFn`] on the async runtime, racing it against a stored shutdown.
fn on_start_server(
	ev: On<StartServer>,
	mut servers: Query<&mut HttpServer>,
	mut keep_alive: ResMut<KeepAlive>,
	mut commands: Commands,
) {
	if !ev.passes("http") {
		return;
	}
	let entity = ev.event_target();
	// resolve the bind config from the event params, the only source of truth.
	if let Ok(mut server) = servers.get_mut(entity) {
		*server = server.clone().with_params(&ev.params);
	}
	keep_alive.acquire();
	// store the shutdown sender on the host; move the receiver into the accept loop.
	let (signal, shutdown) = oneshot::<()>();
	commands
		.entity(entity)
		.insert(HttpServerShutdown(Some(signal)))
		.queue_async_local(move |entity| start_http_server(entity, shutdown));
}

/// Tears down the HTTP backend when a [`StopServer`] passing `"http"` lands: signals
/// the shutdown channel (ending the accept loop and dropping the listener, which
/// closes the port) and releases the server's [`KeepAlive`] ref.
fn on_stop_server(
	ev: On<StopServer>,
	mut shutdowns: Query<&mut HttpServerShutdown>,
	mut keep_alive: ResMut<KeepAlive>,
	mut commands: Commands,
) {
	if !ev.passes("http") {
		return;
	}
	let entity = ev.event_target();
	if let Ok(mut shutdown) = shutdowns.get_mut(entity) {
		// signalling wakes the receiver in `start_http_server`, dropping the backend.
		if let Some(signal) = shutdown.0.take() {
			signal.signal(());
		}
		keep_alive.release();
		commands.entity(entity).remove::<HttpServerShutdown>();
	}
}

/// Invoke the installed backend on a started host, racing its accept loop against
/// the `shutdown` signal. The backend owns its listener, so when a [`StopServer`]
/// signals the channel the race drops the backend future, dropping the listener and
/// closing the port. Skips a host already despawned (eg a serialization spawn).
async fn start_http_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	if !entity.is_alive().await {
		return Ok(());
	}
	let Some(backend) = http_server() else {
		bevybail!(
			"No HTTP server backend installed. Enable a server feature \
			 (server/hyper/lambda) or install one via set_http_server(...)."
		)
	};
	// the shutdown branch resolves when signalled; `or` then drops the backend
	// future, dropping its listener and closing the port.
	beet_core::exports::futures_lite::future::or(backend(entity), async move {
		shutdown.wait().await;
		Result::Ok(())
	})
	.await
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
		/// spawn bundle. The `HttpServer` unit tests do not trigger a
		/// [`StartServer`], so the listener comes from this `OnSpawn`.
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

	/// Install the stub backend: flag the entity, standing in for a real server.
	///
	/// [`set_http_server`] is a process-global [`OnceLock`], so the first install
	/// wins for the whole test binary (notably the single wasm module that runs
	/// every case in series). Every test therefore installs this same idempotent
	/// hook: flagging is observable where a start is expected and harmless where
	/// it is not (a filter miss never invokes the hook).
	fn stub_backend() {
		set_http_server(|entity| {
			Box::pin(async move {
				entity
					.with(|mut entity| {
						entity.insert(ServerStartFlag);
					})
					.await
			})
		})
		.ok();
	}

	/// A reflect insert (the BSX spread path, eg `{(HttpServer{port:8080})}`)
	/// registers the start observer through `on_add`, so a [`StartServer`]
	/// triggered on the host boots it exactly like a regular spawn. With no
	/// server feature here, the installed runtime hook stands in for the backend.
	#[beet_core::test]
	async fn boots_on_triggered_start() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn_empty().id();
		let registry = app.world().resource::<AppTypeRegistry>().clone();
		// reflect-insert `HttpServer`, the same path a BSX `{(HttpServer{..})}`
		// spread takes; the `on_add` registers the start observer through it.
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
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(8080));
		// trigger the start: the http observer queues the backend hook.
		app.world_mut().entity_mut(entity).trigger(StartServer::all);
		app.update_async().await;
		app.world().entity(entity).contains::<ServerStartFlag>().xpect_true();
		// a long-running server holds a `KeepAlive` ref.
		app.world().resource::<KeepAlive>().count().xpect_eq(1);
	}

	/// A [`StopServer`] passing `"http"` releases the server's `KeepAlive` ref and
	/// removes its shutdown handle, so a stopped server no longer holds the process
	/// up (no leaked ref).
	#[beet_core::test]
	async fn stop_releases_keepalive() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn(HttpServer::new(0)).id();
		app.world_mut().entity_mut(entity).trigger(StartServer::all);
		app.update_async().await;
		app.world().resource::<KeepAlive>().count().xpect_eq(1);
		// stop it: the ref drops back to zero and the shutdown handle is gone.
		app.world_mut().entity_mut(entity).trigger(StopServer::all);
		app.update_async().await;
		app.world().resource::<KeepAlive>().count().xpect_eq(0);
		app.world().entity(entity).contains::<HttpServerShutdown>().xpect_false();
	}

	/// Closing the shutdown channel ends the accept loop and drops the listener,
	/// freeing the port: the same race `start_http_server` runs around the backend.
	/// Proves the `StopServer` teardown closes a real listener (it was a no-op
	/// before), so the port reopens.
	#[cfg(all(feature = "server", not(target_arch = "wasm32")))]
	#[beet_core::test]
	async fn shutdown_ends_accept_loop() {
		// a real bound listener; the OS accepts into the backlog so a connect
		// succeeds while the loop runs.
		let listener = async_io::Async::<std::net::TcpListener>::bind(
			core::net::SocketAddr::from(([127, 0, 0, 1], 0)),
		)
		.unwrap();
		let port = listener.get_ref().local_addr().unwrap().port();
		let (signal, shutdown) = oneshot::<()>();
		// mirror `start_http_server`: the accept loop owns the listener, raced
		// against the shutdown receiver.
		let served = beet_core::exports::futures_lite::future::or::<Result<()>, _, _>(
			async move {
				loop {
					listener.accept().await.ok();
				}
				#[allow(unreachable_code)]
				Result::Ok(())
			},
			async move {
				shutdown.wait().await;
				Result::Ok(())
			},
		);
		// open while listening
		std::net::TcpStream::connect(("127.0.0.1", port)).xpect_ok();
		// signal the shutdown: the race resolves, dropping the loser (the loop) and
		// with it the listener.
		signal.signal(());
		served.await.unwrap();
		// the listener is gone, so the port binds afresh.
		std::net::TcpListener::bind(("127.0.0.1", port)).xpect_ok();
	}

	/// `--port` in the [`StartServer`] params overrides the declared component
	/// port before the backend reads the bind address.
	#[beet_core::test]
	async fn resolves_port_from_params() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn(HttpServer::new(8080)).id();
		let mut params = MultiMap::default();
		params.insert("port".into(), "9090".into());
		app.world_mut()
			.entity_mut(entity)
			.trigger(move |entity| StartServer {
				entity,
				filter: default(),
				params,
			});
		app.update_async().await;
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(9090));
	}

	/// A start whose filter does not pass `"http"` leaves the server untouched.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn(HttpServer::new(0)).id();
		app.world_mut().entity_mut(entity).trigger(StartServer::cli);
		app.update_async().await;
		app.world().entity(entity).contains::<ServerStartFlag>().xpect_false();
	}
}

/// Marker the test backend hook inserts in place of binding a port, proving a
/// [`StartServer`] reached the installed backend.
#[cfg(test)]
#[derive(Component)]
struct ServerStartFlag;

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
