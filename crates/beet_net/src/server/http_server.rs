//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_action::prelude::*;
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
/// It is handed an [`AsyncEntity`] for the spawned server and a shutdown
/// [`OnceValueRx`] that resolves when the host's [`Running<Response>`] is removed,
/// and returns a boxed future. The backend reads the [`HttpServer`] config off the entity, opens its
/// own listener, and dispatches each request through `entity.exchange(req)`. It owns
/// its teardown: on the shutdown signal it stops accepting and drops its listener
/// (and may abort tasks it spawned), since only the backend knows how it spawned its
/// own work.
///
/// The future is a [`LocalBoxedFuture`] (never `Send`): the start observer always
/// drives it with `queue_async_local`, so it stays on the thread it was created
/// on. This lets a backend hold a thread-bound resource across an await, eg the
/// lambda backend's tokio runtime [`EnterGuard`](tokio::runtime::EnterGuard).
pub type HttpServerFn =
	fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>;

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

/// The process-global port the canonical [`HttpServer`] is listening on, set by
/// the bind path once a listener binds (see [`HttpServer::set_current_port`]).
///
/// `None` until a canonical server binds. A loopback-bound consumer (eg
/// [`Request::send`] rewriting an authority-less URL, or `beet_ui`'s terminal
/// image fetch) reads it through [`HttpServer::current_port`].
static CURRENT_PORT: RwLock<Option<u16>> = RwLock::new(None);

/// HTTP server that listens for incoming requests, dispatching each through the
/// host's `Action<Request, Response>` dispatch slot via `entity.exchange`.
///
/// A long-running server: the boot fan-out ([`StartRunning<Boot>`]) whose
/// `--server` selects `"http"` boots it through the backend [`ServerPlugin`]
/// installed via [`set_http_server`], reading `--port` / `--host` from the boot
/// request. It never resolves the boot call, so the host's [`Running<Response>`]
/// keep-alive claim persists the process; when that `Running` is removed (a
/// reload or shutdown) its teardown observer stops the listener. A markup-spawned
/// `<Router {(HttpServer{port:0})}>` boots exactly the same way.
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
/// world.spawn((
///     HttpServer::default(),
///     exchange_handler(|req| req.mirror()),
/// )).trigger(StartRunning::boot);
/// ```
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
#[require(ExchangeStats, ContinueRun<Boot, Response>)]
pub struct HttpServer {
	/// The port the server listens on. `None` means the OS will assign
	/// an available port (equivalent to binding to port `0`).
	/// This is ignored by lambda_server.
	pub port: Option<u16>,
	/// The host address to bind to. Defaults to `[127, 0, 0, 1]` (localhost).
	/// Use `[0, 0, 0, 0]` to listen on all interfaces (required for deployed servers).
	pub host: [u8; 4],
	/// Whether this is *the* canonical server: on bind it registers its real
	/// local port as the process [`current_port`](Self::current_port), so an
	/// authority-less [`Request::send`] (and `beet_ui`'s terminal image fetch)
	/// loops back to it. Defaults to `true`; clear it on a secondary listener
	/// that should not claim the loopback port.
	pub canonical: bool,
}

impl Default for HttpServer {
	fn default() -> Self {
		// `env_ext::var` returns "not found" on no_std, so this reads `BEET_HTTP_PORT`
		// / `BEET_HOST` where there is an environment and falls back to the static
		// defaults everywhere else, no feature gate needed.
		let port = resolve_server_port(None);
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
			canonical: true,
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
			canonical: true,
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

	/// The process-global port the canonical server bound (see
	/// [`CURRENT_PORT`]), errors if no canonical server has bound yet.
	///
	/// The OS-assigned port resolves here even for a `port: 0` config, since the
	/// bind path registers the real `local_addr` port, not the configured one.
	pub fn current_port() -> Result<u16> {
		CURRENT_PORT.read().unwrap().ok_or_else(|| {
			bevyhow!("local port not assigned, is the server running yet?")
		})
	}

	/// Register the canonical server's bound port, overwriting any existing
	/// value. Called from the bind path the instant a canonical listener learns
	/// its real `local_addr` (capturing the OS-assigned port for `port: 0`).
	pub fn set_current_port(port: u16) {
		*CURRENT_PORT.write().unwrap() = Some(port);
	}

	/// The socket address to bind, from the component fields (`0` = OS-assigned,
	/// localhost the default host). The boot observer applies any `--port` /
	/// `--host` from the boot request onto these fields before the backend reads
	/// them, so a `--port=8080` overrides a declared `port`.
	pub fn socket_addr(&self) -> core::net::SocketAddr {
		(self.host, self.port.unwrap_or(0)).into()
	}

	/// Overlays `--port` / `--host` from the boot request onto these fields, the
	/// resolved bind config the backend then reads.
	fn apply_request(&mut self, request: &Request) {
		if let Some(port) =
			request.get_param("port").and_then(|val| val.parse().ok())
		{
			self.port = Some(port);
		}
		if let Some(host) = request.get_param("host") {
			self.host = if host == "0.0.0.0" {
				[0, 0, 0, 0]
			} else {
				[127, 0, 0, 1]
			};
		}
	}
}

/// Registers the shared boot + teardown observers on the host (see
/// [`ServerShutdown`]). no_std-clean: the async runtime (`queue_async_local`) and the
/// installed backend hook both build without std.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	ServerShutdown::<HttpServer>::add_observers(&mut world, cx.entity);
}

impl BootServer for HttpServer {
	const SELECTOR: &'static str = "http";

	fn serve(
		entity: AsyncEntity,
		shutdown: OnceValueRx<()>,
	) -> LocalBoxedFuture<'static, Result> {
		Box::pin(start_http_server(entity, shutdown))
	}

	/// `HttpServer` overlays `--port` / `--host` from the boot before the backend
	/// reads the bind address.
	fn apply_boot(&mut self, boot: &Request) { self.apply_request(boot); }
}

/// Invoke the installed backend on a started host, handing it the `shutdown`
/// receiver so it stops accepting and releases its listener when the host's
/// [`Running<Response>`] is removed. Skips a host already despawned (eg a
/// serialization spawn).
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
	backend(entity, shutdown).await
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
		/// spawn bundle. The `HttpServer` unit tests do not boot through the
		/// fan-out, so the listener comes from this `OnSpawn`.
		pub fn new_test<Func, Fut>(run_server: Func) -> (HttpServer, OnSpawn)
		where
			Func: 'static
				+ Send
				+ Sync
				+ FnOnce(
					AsyncEntity,
					async_io::Async<std::net::TcpListener>,
					OnceValueRx<()>,
				) -> Fut,
			Fut: 'static + Send + Sync + Future<Output = Result>,
		{
			let listener = std::net::TcpListener::bind("127.0.0.1:0")
				.expect("failed to bind test server");
			let port = listener.local_addr().unwrap().port();
			let listener = async_io::Async::new(listener)
				.expect("failed to create async listener");
			// these tests never stop the server, so the shutdown sender is dropped:
			// the receiver never resolves and the server runs for the test's duration.
			let (_signal, shutdown) = oneshot::<()>();
			(
				Self {
					port: Some(port),
					..default()
				},
				OnSpawn::new_async(move |entity| {
					run_server(entity, listener, shutdown)
				}),
			)
		}
	}
}

// Generic boot-machinery tests over a stub backend (no real listener). They drive
// to a bounded condition (the stub's flag, the shutdown handle) rather than settling
// a parked server, so they run on native and wasm alike. The real-listener cases
// (eg `shutdown_ends_accept_loop`) bind real TCP and stay native.
#[cfg(test)]
mod tests {
	use super::*;

	/// Install the stub backend: flag the entity, standing in for a real server.
	///
	/// [`set_http_server`] is a process-global [`OnceLock`], so the first install
	/// wins for the whole test binary (notably the single wasm module that runs
	/// every case in series). Every test therefore installs this same idempotent
	/// hook: flagging is observable where a start is expected and harmless where
	/// it is not (a filter miss never invokes the hook).
	fn stub_backend() {
		set_http_server(|entity, _shutdown| {
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

	/// Fire the boot exchange on the host's `ContinueRun<Boot, Response>` slot
	/// (fire-and-forget: the call fans out and parks). `HttpServer` provides that
	/// slot, so the call reaches the http observer exactly as a real boot does.
	fn boot(app: &mut App, port: u16, request: Request) -> Entity {
		let entity = app.world_mut().spawn(HttpServer::new(port)).id();
		app.world_mut().entity_mut(entity).run_async_local(
			move |host| async move {
				host.call::<Boot, Response>(Boot::from(request)).await?;
				Ok(())
			},
		);
		entity
	}

	/// The boot fan-out (no `--server`) reaches the http server: the installed
	/// backend runs and the host parks on its unresolved `Running<Response>`.
	#[beet_core::test]
	async fn boots_on_boot() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 8080, Request::get("/"));
		app_ext::update_until(&mut app, |world| {
			world.entity(entity).contains::<ServerStartFlag>()
		})
		.await
		.xpect_true();
		// a long-running server parks: the boot call's Running is unresolved.
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
	}

	/// Removing the host's `Running<Response>` (a reload, interrupt, or despawn)
	/// fires the teardown observer, which signals the backend's shutdown channel.
	#[beet_core::test]
	async fn teardown_on_running_removed() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 0, Request::get("/"));
		// drive until booted: the shutdown handle holds a live signal.
		app_ext::update_until(&mut app, |world| {
			world
				.entity(entity)
				.get::<ServerShutdown<HttpServer>>()
				.map(|shutdown| shutdown.is_live())
				.unwrap_or(false)
		})
		.await
		.xpect_true();
		// remove the boot's Running: the teardown observer signals shutdown.
		app.world_mut()
			.entity_mut(entity)
			.remove::<Running<Response>>();
		app.update();
		app.world()
			.entity(entity)
			.get::<ServerShutdown<HttpServer>>()
			.unwrap()
			.is_live()
			.xpect_false();
	}

	/// Closing the shutdown channel ends the accept loop and drops the listener,
	/// freeing the port: the same race `start_http_server` runs around the backend.
	/// Proves the teardown closes a real listener, so the port reopens.
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
		let served =
			beet_core::exports::futures_lite::future::or::<Result<()>, _, _>(
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

	/// `--port` in the boot request overrides the declared component port before
	/// the backend reads the bind address.
	#[beet_core::test]
	async fn resolves_port_from_params() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 8080, Request::from_cli_str("--port=9090"));
		// the backend running means `on_action_in` already applied the `--port`.
		app_ext::update_until(&mut app, |world| {
			world.entity(entity).contains::<ServerStartFlag>()
		})
		.await
		.xpect_true();
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(9090));
	}

	/// A boot whose `--server` does not select `"http"` leaves the server untouched.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 0, Request::from_cli_str("--server=cli"));
		// drive a bounded number of frames; the filter miss never flags the entity.
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		app.world()
			.entity(entity)
			.contains::<ServerStartFlag>()
			.xpect_false();
	}
}

/// Marker the test backend hook inserts in place of binding a port, proving the
/// boot fan-out reached the installed backend.
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
			+ FnOnce(
				AsyncEntity,
				async_io::Async<std::net::TcpListener>,
				OnceValueRx<()>,
			) -> Fut,
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

	/// A real running mini server stops when its shutdown signal fires: it serves
	/// before, and after the signal the port is closed (a connect is refused). The
	/// end-to-end proof that the teardown shutdown tears down a live listener,
	/// joining the mechanism (`shutdown_ends_accept_loop`) and the wiring
	/// (`teardown_on_running_removed`).
	#[beet_core::test]
	async fn stops_real_server() {
		let listener = async_io::Async::<std::net::TcpListener>::bind(
			core::net::SocketAddr::from(([127, 0, 0, 1], 0)),
		)
		.unwrap();
		let port = listener.get_ref().local_addr().unwrap().port();
		let url = format!("http://127.0.0.1:{port}");
		// keep the sender in the test so we can stop the server ourselves.
		let (signal, shutdown) = oneshot::<()>();
		let _handle = std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, ServerPlugin))
				.spawn((
					HttpServer {
						port: Some(port),
						..default()
					},
					exchange_handler(|_| Response::ok().with_body("up")),
					OnSpawn::new_async(move |entity| {
						start_mini_http_server_with_tcp(
							entity, listener, shutdown,
						)
					}),
				))
				.run();
		});
		time_ext::sleep_millis(150).await;
		// serving before the stop
		Request::get(&url)
			.send()
			.await
			.unwrap()
			.into_result()
			.await
			.xpect_ok();
		// fire the shutdown: the mini server's race resolves and drops its listener.
		signal.signal(());
		time_ext::sleep_millis(150).await;
		// the port is closed, so a fresh connect is refused.
		Request::get(&url).send().await.xpect_err();
	}
}
