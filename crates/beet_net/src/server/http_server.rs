//! HTTP server component for handling incoming requests.
use crate::prelude::*;
use beet_core::prelude::*;
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
/// The future is a [`LocalBoxedFuture`] (never `Send`): the start entry always
/// drives it as a local task, so it stays on the thread it was created on. This lets a backend hold a thread-bound resource across an await, eg the
/// lambda backend's tokio runtime [`EnterGuard`](tokio::runtime::EnterGuard).
pub type HttpServerFn =
	fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>;

static HTTP_SERVER: OnceLock<HttpServerFn> = OnceLock::new();

/// HTTP server that listens for incoming requests, dispatching each through its
/// host's `Request -> Response` action via `entity.exchange`.
///
/// A long-running server, contributing a start/stop pair to its entity's
/// [`RunningSet`](beet_action::prelude::RunningSet) with its dispatch host as a
/// child. Calling that entity walks
/// the start, which boots this server when `--server` selects `"http"`, through
/// the backend [`ServerPlugin`] installed via [`HttpServer::set_backend`],
/// reading `--port` / `--host` from the boot request. It never resolves the
/// parked call, so the entity's [`Running<Response>`] keep-alive claim persists
/// the process; when that `Running` is removed (a reload or shutdown) the stop
/// entry closes the listener. A markup-spawned `<HttpServer port=0><Router>..
/// </Router></HttpServer>` boots exactly the same way.
///
/// The concrete backend depends on compile-time features:
/// - Default (`server`): lightweight mini HTTP server using `async-io` TCP
/// - `hyper`: full-featured Hyper HTTP server
/// - `lambda`: AWS Lambda runtime
/// - none of the above (eg no_std embedded): a backend installed at runtime via
///   [`HttpServer::set_backend`]
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
#[component(on_add = hook_ext::entity_hook(HttpServer::contribute))]
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

	/// The installed backend, if any.
	pub fn backend() -> Option<HttpServerFn> { HTTP_SERVER.get().copied() }

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
			default_boot: true,
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
	/// localhost the default host). The start entry applies any `--port` /
	/// `--host` from the boot request onto these fields before the backend reads
	/// them, so a `--port=8080` overrides a declared `port`.
	pub fn socket_addr(&self) -> core::net::SocketAddr {
		(self.host, self.port.unwrap_or(0)).into()
	}

	/// This server's [`RunningSet`](beet_action::prelude::RunningSet)
	/// contribution: bind on start when `--server` selects `"http"`, close the
	/// listener on stop.
	fn contribute(entity: &mut EntityCommands) {
		ServerFacet::contribute(entity, HttpServer::boot, |entity, shutdown| {
			Box::pin(start_http_server(entity, shutdown))
		});
	}

	/// Whether this boot selects the server, overlaying its `--port` / `--host`
	/// onto the declared bind config when it does. See [`ServerParams`] for why
	/// the request alone decides.
	fn boot(&mut self, request: &Request) -> Result<bool> {
		if !ServerFilter::selects(request.params(), "http", self.default_boot) {
			return Ok(false);
		}
		let boot = ServerParams::from_request(request)?;
		if let Some(port) = boot.port {
			self.port = Some(port);
		}
		if let Some(host) = boot.host_octets()? {
			self.host = host;
		}
		Ok(true)
	}
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
	let Some(backend) = HttpServer::backend() else {
		bevybail!(
			"No HTTP server backend installed. Enable a server feature \
			 (server/hyper/lambda) or install one via HttpServer::set_backend(...)."
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

// Boot-machinery tests over the stub backend (no real listener), driving to a
// bounded flag rather than settling a parked server. The real-listener cases (eg
// `shutdown_ends_accept_loop`) bind real TCP and stay native.
//
// NATIVE_ONLY: every case that actually *starts* a server is `cfg`-gated off
// wasm. A started server's stub backend and the walk that launched it are two
// tasks doing world bridges, and beet's wasm harness runs every case on one
// single-threaded executor, where resuming one task from inside another's bridge
// re-enters it ("cannot recursively acquire mutex"). No wasm build compiles an
// http backend at all, so what those cases pin is native machinery. See the Phase
// 1 deviations in `.agents/plans/master-plan.md`.
#[cfg(test)]
mod tests {
	use super::*;
	use beet_action::prelude::*;

	/// Fire the boot call on the entity's `RunningSet` action (fire-and-forget:
	/// the call parks and the walk runs). `HttpServer` contributed the only entry,
	/// so the call reaches it exactly as a real boot does.
	fn boot(app: &mut App, port: u16, request: Request) -> Entity {
		let entity = app.world_mut().spawn(HttpServer::new(port)).id();
		call_and_park(app, entity, request);
		entity
	}

	/// Drive until the stub backend reports it started.
	async fn started(app: &mut App) -> bool {
		use bevy::platform::sync::atomic::Ordering;
		app_ext::update_until(app, |_| SERVER_STARTED.load(Ordering::Relaxed))
			.await
	}

	/// Drive a bounded number of frames and assert the stub never started.
	async fn never_started(app: &mut App) {
		use bevy::platform::sync::atomic::Ordering;
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		SERVER_STARTED.load(Ordering::Relaxed).xpect_false();
	}

	/// Call `entity`'s `RunningSet` action and let the result go: a started server
	/// parks the call forever, and a set where every entry declined fails it, so
	/// neither outcome is the assertion.
	fn call_and_park(app: &mut App, entity: Entity, request: Request) {
		app.world_mut().entity_mut(entity).run_async_local(
			move |server| async move {
				server.call::<Request, Response>(request).await.ok();
				Ok(())
			},
		);
	}

	/// The start walk (no `--server`) reaches the http server: the installed
	/// backend runs and the host parks on its unresolved `Running<Response>`.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn boots_on_boot() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 8080, Request::get("/"));
		started(&mut app).await.xpect_true();
		// a long-running server parks: the boot call's Running is unresolved.
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
	}

	/// Removing the host's `Running<Response>` walks the stop entry, which signals
	/// the shutdown the backend is holding, and a despawn is a teardown just the
	/// same: bevy runs remove hooks on despawn, so the stop still reaches the live
	/// listener rather than orphaning it.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn teardown_on_running_removed() {
		use bevy::platform::sync::atomic::Ordering;
		for despawn in [false, true] {
			stub_backend();
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin));
			let entity = boot(&mut app, 0, Request::get("/"));
			started(&mut app).await.xpect_true();
			// end the run either way: the stop entry signals the backend's shutdown.
			match despawn {
				true => app.world_mut().entity_mut(entity).despawn(),
				false => {
					app.world_mut()
						.entity_mut(entity)
						.remove::<Running<Response>>();
				}
			}
			app_ext::update_until(&mut app, |_| {
				SERVER_STOPPED.load(Ordering::Relaxed)
			})
			.await
			.xpect_true();
		}
	}

	/// A serve loop that never opens (a port already bound) fails the run it was
	/// started for, so the load call resolves with the error and the process exits
	/// rather than parking on a server that is not there.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn serve_failure_fails_the_call() {
		use bevy::platform::sync::atomic::Ordering;
		stub_backend();
		STUB_FAILS.store(true, Ordering::Relaxed);
		let caught = Store::<Option<String>>::default();
		let recorder = caught;
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app.world_mut().spawn(HttpServer::new(0)).id();
		app.world_mut()
			.entity_mut(entity)
			.run_async_local(move |server| async move {
				if let Err(err) =
					server.call::<Request, Response>(Request::get("/")).await
				{
					recorder.set(Some(err.to_string()));
				}
				Ok(())
			});
		let settled = app_ext::update_until(&mut app, |_| caught.get().is_some())
			.await;
		STUB_FAILS.store(false, Ordering::Relaxed);
		settled.xpect_true();
		caught.get().unwrap().xpect_contains("Address already in use");
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

	/// `--port` in the boot request overrides the declared component port before
	/// the backend reads the bind address.
	#[cfg(not(target_arch = "wasm32"))] // see NATIVE_ONLY
	#[beet_core::test]
	async fn resolves_port_from_params() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = boot(&mut app, 8080, Request::from_cli_str("--port=9090"));
		// the backend running means the start entry already applied the `--port`.
		started(&mut app).await.xpect_true();
		app.world()
			.entity(entity)
			.get::<HttpServer>()
			.unwrap()
			.port
			.xpect_eq(Some(9090));
	}

	/// A start whose `--server` does not select `"http"` leaves the server
	/// untouched. The lone declining entry starts nothing, so the call itself
	/// fails (see `unselected_boot_exits`); the assertion here is only that the
	/// backend never ran.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		boot(&mut app, 0, Request::from_cli_str("--server=cli"));
		never_started(&mut app).await;
	}

	/// A server with `default_boot: false` stays dormant on a bare boot (no
	/// `--server`), where the default `default_boot: true` (see `boots_on_boot`)
	/// would start it. As in `skips_on_filter_miss` the call itself fails, having
	/// started nothing.
	#[beet_core::test]
	async fn default_boot_false_skips_bare_boot() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, ServerPlugin));
		let entity = app
			.world_mut()
			.spawn(HttpServer {
				default_boot: false,
				..default()
			})
			.id();
		call_and_park(&mut app, entity, Request::get("/"));
		// a bare boot selects `default_boot` servers only; this one opts out.
		never_started(&mut app).await;
	}
}

/// Whether the stub backend started, in place of binding a port, and whether its
/// shutdown then resolved, in place of dropping a listener: the two observable
/// ends of the start/stop path.
///
/// Process flags rather than component markers, because the stub stands in for a
/// serve loop and a serve loop must be able to run without touching the world:
/// its shutdown can arrive mid-command (a despawn teardown), where there is no
/// entity left to mark and no world access to be had.
#[cfg(test)]
pub(crate) static SERVER_STARTED: bevy::platform::sync::atomic::AtomicBool =
	bevy::platform::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
pub(crate) static SERVER_STOPPED: bevy::platform::sync::atomic::AtomicBool =
	bevy::platform::sync::atomic::AtomicBool::new(false);

/// Makes the stub backend fail instead of starting, standing in for a bind
/// failure. Global rather than per-case because [`HttpServer::set_backend`] is a
/// process-global first-install-wins hook; a case sets it, drives, and clears it.
#[cfg(test)]
pub(crate) static STUB_FAILS: bevy::platform::sync::atomic::AtomicBool =
	bevy::platform::sync::atomic::AtomicBool::new(false);

/// Install the shared test backend and reset its flags: raise [`SERVER_STARTED`]
/// in place of binding, then await the shutdown and raise [`SERVER_STOPPED`] in
/// place of dropping the listener.
///
/// [`HttpServer::set_backend`] is a process-global [`OnceLock`], so the first
/// install wins for the whole test binary (notably the single wasm module that
/// runs every case in series). Every test that starts a server calls this same
/// idempotent installer, so cases stay order-independent.
#[cfg(test)]
pub(crate) fn stub_backend() {
	use bevy::platform::sync::atomic::Ordering;
	SERVER_STARTED.store(false, Ordering::Relaxed);
	SERVER_STOPPED.store(false, Ordering::Relaxed);
	HttpServer::set_backend(|_entity, shutdown| {
		Box::pin(async move {
			if STUB_FAILS.load(Ordering::Relaxed) {
				bevybail!("Failed to bind stub server: Address already in use");
			}
			SERVER_STARTED.store(true, Ordering::Relaxed);
			shutdown.wait().await;
			SERVER_STOPPED.store(true, Ordering::Relaxed);
			Ok(())
		})
	})
	.ok();
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
			app.world_mut().spawn((server, children![exchange_ext::handler(
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
				children![exchange_ext::handler(|_| Response::ok().with_body("up"))],
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
