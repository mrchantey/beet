//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::OnceLock;

/// Boxed server-start function: the no_std-friendly server hook, mirroring
/// [`HttpSendFn`] on the client side.
///
/// [`ServerPlugin`] installs one of the built-in backends (mini / hyper / lambda)
/// via [`HttpServer::set_backend`] based on compile-time features; a downstream adapter
/// (an embassy / esp WiFi crate, …) installs its own without living in
/// [`beet_net`]. [`HttpServer`]'s start entry invokes the installed function.
///
/// It is handed an [`AsyncEntity`] for the spawned server and a shutdown
/// [`OnceValueRx`] that resolves when the entity's [`Running<Response>`] is removed,
/// and returns a boxed future. The backend reads the [`HttpServer`] config off the entity, opens its
/// own listener, and dispatches each request through `entity.exchange(req)`,
/// which resolves the dispatch host (usually the server's parent). It owns
/// its teardown: on the shutdown signal it stops accepting and drops its listener
/// (and may abort tasks it spawned), since only the backend knows how it spawned its
/// own work.
///
/// The future is a [`LocalBoxedFuture`] (never `Send`): the facet always drives it
/// as a local task, so it stays on the thread it was created on. This lets a backend hold a thread-bound resource across an await, eg the
/// lambda backend's tokio runtime [`EnterGuard`](tokio::runtime::EnterGuard).
pub type HttpServerFn =
	fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>;

/// One [`HttpServer`]'s own backend, outranking the process-global
/// [`HttpServerFn`] install for that entity alone.
///
/// A closure rather than [`HttpServerFn`]'s fn pointer, so a caller can capture
/// per-instance state: several servers in one process (or one test case) each
/// serve their own way without racing over the global [`OnceLock`].
pub type HttpServerBackend = Arc<
	dyn Fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>
		+ Send
		+ Sync,
>;

static HTTP_SERVER: OnceLock<HttpServerFn> = OnceLock::new();

/// HTTP server that listens for incoming requests, dispatching each through its
/// host's `Request -> Response` action via `entity.exchange`.
///
/// A long-running server, adding one facet to its entity's
/// [`RunningSet`](beet_action::prelude::RunningSet) with its dispatch host as a
/// child. Calling that entity starts this facet when `--server` selects
/// `"http"`, running the backend [`ServerPlugin`] installed via
/// [`HttpServer::set_backend`] and reading `--port` / `--host` from the start
/// request. It never resolves the parked call, so the entity's
/// [`Running<Response>`] keep-alive claim persists the process; when that
/// `Running` is removed (a reload or shutdown) the facet's shutdown signal fires
/// and the listener closes. A markup-spawned `<HttpServer port=0><Router>..
/// </Router></HttpServer>` boots exactly the same way.
///
/// The concrete backend depends on compile-time features:
/// - Default (`server`): lightweight mini HTTP server using `async-io` TCP
/// - `hyper`: full-featured Hyper HTTP server
/// - `lambda`: AWS Lambda runtime
/// - none of the above (eg no_std embedded): a backend installed at runtime via
///   [`HttpServer::set_backend`], or per-entity via [`HttpServer::backend`]
///
/// # Example
///
/// ```ignore
/// # use beet_core::prelude::*;
/// # use beet_net::prelude::*;
/// let mut world = World::new();
/// world.spawn((
///     HttpServer::default(),
///     LoadRequest::from_cli().on_spawn(),
///     children![exchange_ext::handler(|req| req.mirror())],
/// ));
/// ```
#[derive(Clone, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = hook_ext::component_hook(HttpServer::add_facet))]
#[require(ExchangeStats)]
pub struct HttpServer {
	/// The port the server listens on. `None` means the OS will assign
	/// an available port (equivalent to binding to port `0`).
	/// This is ignored by lambda_server.
	pub port: Option<u16>,
	/// The host address to bind to. Defaults to `[127, 0, 0, 1]` (localhost).
	/// Use `[0, 0, 0, 0]` to listen on all interfaces (required for deployed servers).
	pub host: [u8; 4],
	/// Whether this is *the* canonical server: on bind it registers its real
	/// local port as the process [`CanonicalPort`], so an authority-less
	/// [`Request::send`] (and `beet_ui`'s terminal image fetch) loops back to it.
	/// Defaults to `true`; clear it on a secondary listener that should not claim
	/// the loopback port.
	pub canonical: bool,
	/// Whether a bare `beet` (no `--server`) boots this server. `true` by default,
	/// so an entry declaring a single [`HttpServer`] needs no flag; clear it on a
	/// server that should boot only when `--server=http` names it explicitly.
	pub default_boot: bool,
	/// This server's own backend, outranking the process-global
	/// [`HttpServer::set_backend`] install. `None` (the default) falls back to it.
	///
	/// Not markup-declarable (it holds a closure), so it is set by whoever spawns
	/// the entity: a second server in one process, or a test standing a listener
	/// in without racing the first-install-wins global.
	#[reflect(ignore)]
	pub backend: Option<HttpServerBackend>,
}

impl Default for HttpServer {
	/// Reads the process [`BootstrapConfig`] (`--port` / `BEET_HTTP_PORT` and
	/// `--host` / `BEET_HOST`), falling back to [`DEFAULT_HTTP_PORT`] on
	/// localhost. Both sources are empty where there is no process environment
	/// or argv, so no feature gate is needed.
	fn default() -> Self {
		let config = BootstrapConfig::get();
		Self {
			port: Some(resolve_server_port(None)),
			host: config.host_octets().unwrap_or([127, 0, 0, 1]),
			canonical: true,
			default_boot: true,
			backend: None,
		}
	}
}

impl HttpServer {
	/// Install the backend [`HttpServer`] invokes on start. [`ServerPlugin`] calls
	/// this for the compile-time-selected feature backend; a no_std adapter with no
	/// compiled-in backend installs its own. Returns an error if one is already set.
	pub fn set_backend(server: HttpServerFn) -> Result<()> {
		HTTP_SERVER
			.set(server)
			.map_err(|_| bevyhow!("HTTP server already installed"))
	}

	/// The process-global installed backend, if any.
	pub fn installed_backend() -> Option<HttpServerFn> {
		HTTP_SERVER.get().copied()
	}

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
			..default()
		}
	}
	/// Serve through `backend` rather than the process-global install, so this
	/// one server owns how it listens.
	pub fn with_backend<Func>(mut self, backend: Func) -> Self
	where
		Func: 'static
			+ Send
			+ Sync
			+ Fn(
				AsyncEntity,
				OnceValueRx<()>,
			) -> LocalBoxedFuture<'static, Result>,
	{
		self.backend = Some(Arc::new(backend));
		self
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
	/// localhost the default host). The facet applies any `--port` / `--host`
	/// from the start request onto these fields before the backend reads them, so
	/// a `--port=8080` overrides a declared `port`.
	pub fn socket_addr(&self) -> core::net::SocketAddr {
		(self.host, self.port.unwrap_or(0)).into()
	}

	/// This server's [`RunningSet`](beet_action::prelude::RunningSet) facet: bind
	/// when `--server` selects `"http"`, hold the listener open until the shutdown
	/// signal, then drop it.
	fn add_facet(&self) -> impl FnOnce(&mut EntityCommands) + use<> {
		// selection and backend are read once here, when the server is declared,
		// so the facet decides without a world access.
		let default_boot = self.default_boot;
		let backend = self.backend.clone();
		move |entity: &mut EntityCommands| {
			RunningSet::<Request, Response>::add(
				entity,
				"http",
				move |request: &Request| {
					RunningSetFilter::selects(
						request.params(),
						"http",
						default_boot,
					)
				},
				move |entity, request, shutdown| {
					// the bind knobs are the request's; the future owns them, so
					// nothing borrows the input past this call.
					let params = ServerParams::from_request(request);
					let backend = backend.clone();
					Box::pin(serve_http(entity, backend, params, shutdown))
				},
			);
		}
	}
}

/// Overlay the start request's bind knobs onto the declared config, then invoke
/// this server's backend, handing it the `shutdown` receiver so it stops
/// accepting and releases its listener when the host's [`Running<Response>`] is
/// removed.
///
/// Skips a host already despawned (eg a serialization spawn). Errors propagate to
/// the driver, so a listener that never opens (a port already bound) fails the
/// parked call rather than raising into the app's error handler.
async fn serve_http(
	entity: AsyncEntity,
	backend: Option<HttpServerBackend>,
	params: Result<ServerParams>,
	shutdown: OnceValueRx<()>,
) -> Result {
	if !entity.is_alive().await {
		return Ok(());
	}
	let params = params?;
	let host = params.host_octets()?;
	entity
		.get_mut::<HttpServer, _>(move |mut server| {
			if let Some(port) = params.port {
				server.port = Some(port);
			}
			if let Some(host) = host {
				server.host = host;
			}
		})
		.await?;
	// the entity's own backend outranks the process-global install, so a second
	// server (or a test) serves its own way without racing that `OnceLock`.
	match backend {
		Some(backend) => backend(entity, shutdown).await,
		None => match HttpServer::installed_backend() {
			Some(backend) => backend(entity, shutdown).await,
			None => bevybail!(
				"No HTTP server backend installed. Enable a server feature \
				 (server/hyper/lambda) or install one via \
				 HttpServer::set_backend(...)."
			),
		},
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
			let (_signal, shutdown) = OnceValue::<()>::oneshot();
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

// Facet-machinery tests over a per-entity stub backend (no real listener),
// driving to a bounded log rather than settling a parked server. The
// real-listener cases (eg `shutdown_ends_accept_loop`) bind real TCP and stay
// native.
//
// NATIVE_ONLY: every case that actually *starts* a server is `cfg`-gated off
// wasm. A started server's stub backend and the driver that launched it are two
// tasks doing world bridges, and beet's wasm harness runs every case on one
// single-threaded executor, where resuming one task from inside another's bridge
// re-enters it ("cannot recursively acquire mutex"). No wasm build compiles an
// http backend at all, so what those cases pin is native machinery. See Phase 4
// of `.agents/plans/lifecycle-master-plan.md`, which lifts these gates.
#[cfg(test)]
pub(crate) mod tests {
	use super::*;

	/// A server whose backend records its two observable ends in `log`: `"start"`
	/// in place of binding a port, `"stop"` in place of dropping the listener.
	///
	/// Per-entity rather than the process-global install, so concurrently-driven
	/// cases never observe each other's servers.
	pub(crate) fn stub_server(
		port: u16,
		log: Store<Vec<&'static str>>,
	) -> HttpServer {
		HttpServer::new(port).with_backend(move |_entity, shutdown| {
			Box::pin(async move {
				log.push("start");
				shutdown.wait().await;
				log.push("stop");
				Ok(())
			})
		})
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		app
	}

	/// Spawn a stub server and call it, returning the entity and its log.
	fn boot(
		app: &mut App,
		port: u16,
		request: Request,
	) -> (Entity, Store<Vec<&'static str>>) {
		let log = Store::<Vec<&'static str>>::default();
		let entity = app.world_mut().spawn(stub_server(port, log)).id();
		call_and_park(app, entity, request);
		(entity, log)
	}

	/// Drive until `log` holds `len` entries, failing fast rather than hanging.
	async fn until_logged(
		app: &mut App,
		log: Store<Vec<&'static str>>,
		len: usize,
	) -> bool {
		app_ext::update_until(app, |_| log.len() >= len).await
	}

	/// Drive a bounded number of frames and assert the stub never started.
	async fn never_started(app: &mut App, log: Store<Vec<&'static str>>) {
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		log.get().xpect_eq(Vec::<&'static str>::new());
	}

	/// Call `entity`'s `RunningSet` action and let the result go: a started server
	/// parks the call forever, and a start every facet declined fails it, so
	/// neither outcome is the assertion.
	pub(crate) fn call_and_park(
		app: &mut App,
		entity: Entity,
		request: Request,
	) {
		app.world_mut().entity_mut(entity).run_async_local(
			move |server| async move {
				server.call::<Request, Response>(request).await.ok();
				Ok(())
			},
		);
	}

	/// A bare start (no `--server`) selects the http facet: its backend runs and
	/// the host parks on its unresolved `Running<Response>`.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn boots_on_boot() {
		let mut app = app();
		let (entity, log) = boot(&mut app, 8080, Request::get("/"));
		until_logged(&mut app, log, 1).await.xpect_true();
		// a long-running server parks: the boot call's Running is unresolved.
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
	}

	/// Removing the host's `Running<Response>` fires the facet's shutdown signal,
	/// and a despawn is a teardown just the same: bevy runs remove hooks on
	/// despawn, so the signal still reaches the live listener rather than
	/// orphaning it.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn teardown_on_running_removed() {
		for despawn in [false, true] {
			let mut app = app();
			let (entity, log) = boot(&mut app, 0, Request::get("/"));
			until_logged(&mut app, log, 1).await.xpect_true();
			// end the run either way: the removal signals the facet's shutdown.
			match despawn {
				true => app.world_mut().entity_mut(entity).despawn(),
				false => {
					app.world_mut()
						.entity_mut(entity)
						.remove::<Running<Response>>();
				}
			}
			until_logged(&mut app, log, 2).await.xpect_true();
			log.get().xpect_eq(vec!["start", "stop"]);
		}
	}

	/// A serve loop that never opens (a port already bound) fails the run it was
	/// started for, so the load call resolves with the error and the process exits
	/// rather than parking on a server that is not there.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn serve_failure_fails_the_call() {
		let caught = Store::<Option<String>>::default();
		let mut app = app();
		let entity = app
			.world_mut()
			.spawn(HttpServer::new(0).with_backend(|_entity, _shutdown| {
				Box::pin(async move {
					bevybail!(
						"Failed to bind stub server: Address already in use"
					)
				})
			}))
			.id();
		app.world_mut().entity_mut(entity).run_async_local(
			move |server| async move {
				if let Err(err) =
					server.call::<Request, Response>(Request::get("/")).await
				{
					caught.set(Some(err.to_string()));
				}
				Ok(())
			},
		);
		app_ext::update_until(&mut app, |_| caught.get().is_some())
			.await
			.xpect_true();
		caught
			.get()
			.unwrap()
			.xpect_contains("Address already in use");
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
		let (signal, shutdown) = OnceValue::<()>::oneshot();
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

	/// `--port` in the start request overrides the declared component port before
	/// the backend reads the bind address.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn resolves_port_from_params() {
		let mut app = app();
		let (entity, log) =
			boot(&mut app, 8080, Request::from_cli_str("--port=9090"));
		// the backend running means the facet already applied the `--port`.
		until_logged(&mut app, log, 1).await.xpect_true();
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(9090));
	}

	/// A start whose `--server` does not select `"http"` leaves the server
	/// untouched. The lone declining facet starts nothing, so the call itself
	/// fails (see `unselected_boot_exits`); the assertion here is only that the
	/// backend never ran.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		let mut app = app();
		let (_entity, log) =
			boot(&mut app, 0, Request::from_cli_str("--server=cli"));
		never_started(&mut app, log).await;
	}

	/// A server with `default_boot: false` stays dormant on a bare start (no
	/// `--server`), where the default `default_boot: true` (see `boots_on_boot`)
	/// would start it. As in `skips_on_filter_miss` the call itself fails, having
	/// started nothing.
	#[beet_core::test]
	async fn default_boot_false_skips_bare_boot() {
		let mut app = app();
		let log = Store::<Vec<&'static str>>::default();
		let entity = app
			.world_mut()
			.spawn(HttpServer {
				default_boot: false,
				..stub_server(0, log)
			})
			.id();
		call_and_park(&mut app, entity, Request::get("/"));
		// a bare start selects `default_boot` servers only; this one opts out.
		never_started(&mut app, log).await;
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
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			// the server owns the boot, its dispatch host is the child
			app.world_mut()
				.spawn((server, children![exchange_ext::handler(
					move |req| Response::ok().with_body(req.take().body)
				)]));
			app.run();
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
		let (signal, shutdown) = OnceValue::<()>::oneshot();
		let _handle = std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			app.world_mut().spawn((
				HttpServer {
					port: Some(port),
					..default()
				},
				OnSpawn::new_async(move |entity| {
					HttpServer::start_mini_with_tcp(entity, listener, shutdown)
				}),
				// the server's dispatch host, a child
				children![exchange_ext::handler(
					|_| Response::ok().with_body("up")
				)],
			));
			app.run();
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
