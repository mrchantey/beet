//! WebSocket server component, aligned with [`HttpServer`]'s installable-backend
//! boot model.
use crate::prelude::*;
use crate::sockets::PersistentSocket;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::OnceLock;

/// Boxed socket-server-start function: the no_std-friendly server hook, mirroring
/// [`HttpServerFn`].
///
/// [`SocketServerPlugin`] installs the built-in tungstenite backend via
/// [`set_socket_server`] when the feature is enabled; a downstream adapter (an
/// embassy / esp WiFi crate, …) installs its own without living in [`beet_net`].
/// [`SocketServer`]'s start observer invokes the installed function.
///
/// The seam matches [`HttpServerFn`] exactly so the same boot/teardown machinery
/// drives both: it is handed an [`AsyncEntity`] for the spawned server and a
/// shutdown [`OnceValueRx`] that resolves when the host's [`Running<Response>`] is
/// removed, and returns a boxed future. The backend reads the [`SocketServer`]
/// config off the entity, opens its own listener, adopts each accepted connection
/// as a child [`Socket`], and owns its teardown on the shutdown signal.
///
/// The future is a [`LocalBoxedFuture`] (never `Send`): the accept loop and its
/// per-connection [`Socket`] readers are thread-bound, so the start observer always
/// drives it with `queue_async_local`.
pub type SocketServerFn =
	fn(AsyncEntity, OnceValueRx<()>) -> LocalBoxedFuture<'static, Result>;

static SOCKET_SERVER: OnceLock<SocketServerFn> = OnceLock::new();

/// Install the backend [`SocketServer`] invokes on start. [`SocketServerPlugin`]
/// calls this for the compile-time tungstenite backend; a no_std adapter with no
/// compiled-in backend installs its own. Returns an error if one is already set.
pub fn set_socket_server(server: SocketServerFn) -> Result<()> {
	SOCKET_SERVER
		.set(server)
		.map_err(|_| bevyhow!("Socket server already installed"))
}

/// The installed backend, if any.
pub fn socket_server() -> Option<SocketServerFn> {
	SOCKET_SERVER.get().copied()
}

/// Plugin for running bevy WebSocket servers.
///
/// Registers reflection for [`SocketServer`] and installs the compile-time
/// tungstenite backend via [`set_socket_server`], mirroring [`ServerPlugin`].
#[derive(Default)]
pub struct SocketServerPlugin;

impl Plugin for SocketServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>()
			.register_type::<SocketServer>()
			.register_type::<PersistentSocket>()
			.register_type::<Tls>();

		// install the tungstenite backend; mirrors `ServerPlugin`. Skipped in
		// beet_net's own tests, which install a stub per case. `set_socket_server`
		// errors if one is already installed, so ignore a re-install.
		#[cfg(not(test))]
		{
			cfg_if! {
				if #[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))] {
					set_socket_server(|entity, shutdown| {
						Box::pin(super::start_tungstenite_server(entity, shutdown))
					})
					.ok();
				} else {
					// no feature backend: a downstream `set_socket_server` supplies one.
				}
			}
		}
	}
}

/// A WebSocket server that accepts incoming connections, booting through the same
/// fan-out as [`HttpServer`].
///
/// The boot fan-out ([`StartRunning<Boot>`]) whose `--server` selects `"socket"`
/// boots it through the backend installed via [`set_socket_server`]. It never
/// resolves the boot call, so the host's [`Running<Response>`] keep-alive parks the
/// process; when that `Running` is removed (a reload or shutdown) its teardown
/// observer signals the backend to stop accepting and drop its listener. Each
/// accepted connection is adopted as a child [`Socket`], dispatching via the
/// entity's [`MessageRecv`] / [`MessageSend`] events.
///
/// The concrete backend depends on compile-time features:
/// - `tungstenite` (native): an `async-io` TCP WebSocket listener
/// - none of the above (eg no_std embedded): a backend installed at runtime via
///   [`set_socket_server`]
#[derive(Clone, Debug, Component, Reflect)]
#[reflect(Component, Default)]
#[component(on_add = on_add)]
#[require(ContinueRun<Boot, Response>)]
pub struct SocketServer {
	/// The port to bind to. `None` means the OS will assign a port.
	pub port: Option<u16>,
	/// The IPv4 interface to bind to, defaulting to `0.0.0.0` (all interfaces) so a
	/// remote host - a browser tab, an esp device over Wi-Fi - can connect. Narrow it
	/// with [`with_host`](Self::with_host), eg `[127, 0, 0, 1]` to bind loopback only.
	pub host: [u8; 4],
}

impl Default for SocketServer {
	fn default() -> Self { Self::new(DEFAULT_SOCKET_PORT) }
}

impl SocketServer {
	/// Creates a new socket server on `port`, bound to all interfaces (`0.0.0.0`) so a
	/// browser or a device on the network can reach it. Narrow with
	/// [`with_host`](Self::with_host), eg `[127, 0, 0, 1]` for loopback only.
	pub fn new(port: u16) -> Self {
		Self {
			port: Some(port),
			host: [0, 0, 0, 0],
		}
	}

	/// Bind to `host` (an IPv4 interface) instead of loopback, so remote hosts can
	/// connect. Pair with a routable address, ie `[0, 0, 0, 0]` for all interfaces.
	pub fn with_host(mut self, host: [u8; 4]) -> Self {
		self.host = host;
		self
	}

	/// Bind to `0.0.0.0` (all interfaces) - the [`new`](Self::new) default, kept as an
	/// explicit spelling for a server that wants to state its reach.
	pub fn bind_all(self) -> Self { self.with_host([0, 0, 0, 0]) }

	/// The host and port without the protocol, ie `127.0.0.1:3000`
	pub fn local_address(&self) -> String {
		let [a, b, c, d] = self.host;
		let port = self.port.unwrap_or(0);
		format!("{a}.{b}.{c}.{d}:{port}")
	}
	/// Returns the full WebSocket URL for local connections, e.g. `ws://127.0.0.1:8339`.
	pub fn local_url(&self) -> String {
		format!("ws://{}", self.local_address())
	}

	/// Creates a test server bound to an OS-assigned port, alongside the [`OnSpawn`]
	/// that starts its pre-bound listener.
	///
	/// Binds to port `0` so the OS picks a free port, avoiding collisions in
	/// parallel tests; the listener is kept alive and handed straight to the
	/// backend, eliminating the bind/connect race. These tests start the listener
	/// from the returned [`OnSpawn`] rather than booting through the fan-out, so the
	/// shutdown sender is dropped and the server runs for the test's duration.
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	pub fn new_test() -> (SocketServer, OnSpawn) {
		let listener = std::net::TcpListener::bind("127.0.0.1:0")
			.expect("failed to bind test socket server");
		let port = listener.local_addr().unwrap().port();
		let listener = async_io::Async::new(listener)
			.expect("failed to create async listener");
		let (_signal, shutdown) = oneshot::<()>();
		(
			Self {
				port: Some(port),
				host: [127, 0, 0, 1],
			},
			OnSpawn::new_async_local(move |entity| {
				super::start_tungstenite_server_with_tcp(
					entity, listener, shutdown,
				)
			}),
		)
	}
}

/// Registers the shared boot + teardown observers, mirroring [`HttpServer`] (see
/// [`ServerShutdown`]).
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	ServerShutdown::<SocketServer>::add_observers(&mut world, cx.entity);
}

impl BootServer for SocketServer {
	const SELECTOR: &'static str = "socket";

	fn serve(
		entity: AsyncEntity,
		shutdown: OnceValueRx<()>,
	) -> LocalBoxedFuture<'static, Result> {
		Box::pin(start_socket_server(entity, shutdown))
	}
}

/// Invoke the installed backend on a started host, handing it the `shutdown`
/// receiver so it stops accepting and releases its listener when the host's
/// [`Running<Response>`] is removed. Skips a host already despawned.
async fn start_socket_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	if !entity.is_alive().await {
		return Ok(());
	}
	let Some(backend) = socket_server() else {
		bevybail!(
			"No socket server backend installed. Enable the tungstenite feature \
			 or install one via set_socket_server(...)."
		)
	};
	backend(entity, shutdown).await
}

// Generic boot-machinery tests over a stub backend (no real listener), so they run
// on native and wasm alike. The real-listener tests below bind real TCP and stay
// native + tungstenite.
#[cfg(test)]
mod tests {
	use super::*;

	/// Marker the stub backend inserts in place of binding a listener, proving the
	/// boot fan-out reached the installed backend.
	#[derive(Component)]
	struct SocketStartFlag;

	/// Install the stub backend: flag the entity, standing in for a real server.
	///
	/// [`set_socket_server`] is a process-global [`OnceLock`], so the first install
	/// wins for the whole test binary (notably the single wasm module that runs
	/// every case in series). Every test therefore installs this same idempotent
	/// hook: flagging is observable where a start is expected and harmless where it
	/// is not (a filter miss never invokes the hook).
	fn stub_backend() {
		set_socket_server(|entity, _shutdown| {
			Box::pin(async move {
				entity
					.with(|mut entity| {
						entity.insert(SocketStartFlag);
					})
					.await
			})
		})
		.ok();
	}

	/// Fire the boot exchange on the host's `ContinueRun<Boot, Response>` slot
	/// (fire-and-forget: the call fans out and parks).
	fn boot(app: &mut App, request: Request) -> Entity {
		let entity = app.world_mut().spawn(SocketServer::default()).id();
		app.world_mut().entity_mut(entity).run_async_local(
			move |host| async move {
				host.call::<Boot, Response>(Boot::from(request)).await?;
				Ok(())
			},
		);
		entity
	}

	/// The boot fan-out reaches the socket server: the installed backend runs and
	/// the host parks on its unresolved `Running<Response>`.
	#[beet_core::test]
	async fn boots_on_boot() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, SocketServerPlugin::default()));
		let entity = boot(&mut app, Request::get("/"));
		app_ext::update_until(&mut app, |world| {
			world.entity(entity).contains::<SocketStartFlag>()
		})
		.await
		.xpect_true();
		// a long-running server parks: the boot call's Running is unresolved.
		app.world()
			.entity(entity)
			.contains::<Running<Response>>()
			.xpect_true();
	}

	/// Removing the host's `Running<Response>` fires the teardown observer, which
	/// signals the backend's shutdown channel.
	#[beet_core::test]
	async fn teardown_on_running_removed() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, SocketServerPlugin::default()));
		let entity = boot(&mut app, Request::get("/"));
		// drive until booted: the shutdown handle holds a live signal.
		app_ext::update_until(&mut app, |world| {
			world
				.entity(entity)
				.get::<ServerShutdown<SocketServer>>()
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
			.get::<ServerShutdown<SocketServer>>()
			.unwrap()
			.is_live()
			.xpect_false();
	}

	/// A boot whose `--server` does not select `"socket"` leaves the server
	/// untouched.
	#[beet_core::test]
	async fn skips_on_filter_miss() {
		stub_backend();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, SocketServerPlugin::default()));
		let entity = boot(&mut app, Request::from_cli_str("--server=cli"));
		// drive a bounded number of frames; the filter miss never flags the entity.
		for _ in 0..16 {
			app.update();
			AsyncRunner::tick().await;
		}
		app.world()
			.entity(entity)
			.contains::<SocketStartFlag>()
			.xpect_false();
	}
}

// Bind real TCP, so native + tungstenite only; the wasm-runnable socket-server
// coverage is the `ChannelSocketServer` end-to-end test (see `channel_socket_server`).
#[cfg(all(test, feature = "tungstenite", not(target_arch = "wasm32")))]
mod real_listener_tests {
	use super::*;
	use crate::sockets::Message;
	use crate::sockets::*;

	#[beet_core::test]
	async fn server_binds_and_accepts() {
		let server = SocketServer::new_test();
		let url = server.0.local_url();

		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, SocketServerPlugin::default()))
				.spawn(server)
				.run();
		});
		time_ext::sleep_millis(200).await;

		let mut client = Socket::connect(&url).await.unwrap();
		client.send(Message::text("hello server")).await.unwrap();
		client.close(None).await.ok();
	}

	#[beet_core::test]
	async fn handles_multiple_concurrent_connections() {
		let server = SocketServer::new_test();
		let url = server.0.local_url();

		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, SocketServerPlugin::default()))
				.spawn(server)
				.run();
		});
		time_ext::sleep_millis(200).await;

		let mut client_one = Socket::connect(&url).await.unwrap();
		client_one.send(Message::text("client1")).await.unwrap();

		let mut client_two = Socket::connect(&url).await.unwrap();
		client_two.send(Message::text("client2")).await.unwrap();

		client_one.close(None).await.ok();
		client_two.close(None).await.ok();
	}

	/// Common sockets workflow:
	///
	/// 1. client sends text to server
	/// 2. server echoes text back
	/// 3. client sends close to server
	/// 4. server sends close back
	#[beet_core::test]
	async fn ecs_sockets() {
		let server = SocketServer::new_test();
		let url = server.0.local_url();
		let store = Store::<bool>::default();

		let _handle = std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SocketServerPlugin::default()));

			// spawn server with echo observer
			app.world_mut().spawn(server).observe_any(
				|ev: On<MessageRecv>, mut commands: Commands| match ev
					.event()
					.inner()
				{
					Message::Text(text) => {
						commands.entity(ev.original_target()).trigger_target(
							MessageSend(Message::Text(text.clone())),
						);
					}
					Message::Close(_) => {
						commands
							.entity(ev.original_target())
							.trigger_target(MessageSend(Message::Close(None)));
					}
					_ => {}
				},
			);

			// spawn client with ready and recv observers
			app.world_mut()
				.spawn(Socket::insert_on_connect(&url))
				.observe_any(|ev: On<SocketReady>, mut commands: Commands| {
					commands.entity(ev.target()).trigger_target(MessageSend(
						Message::Text("hello matey".into()),
					));
				})
				.observe_any(
					move |ev: On<MessageRecv>, mut commands: Commands| match ev
						.event()
						.inner()
					{
						Message::Text(text) => {
							text.xpect_eq("hello matey");
							commands
								.entity(ev.original_target())
								.trigger_target(MessageSend(Message::Close(
									None,
								)));
						}
						Message::Close(_) => {
							store.set(true);
							commands.write_message(AppExit::Success);
						}
						_ => {}
					},
				);

			app.run();
		});

		// poll the store until the app signals completion
		for _ in 0..100 {
			time_ext::sleep_millis(50).await;
			if store.get() {
				break;
			}
		}
		store.get().xpect_true();
	}
}
