use crate::prelude::DEFAULT_SSH_PORT;
use beet_core::prelude::*;

/// Plugin for running bevy SSH servers.
#[derive(Default)]
pub struct SshServerPlugin;

impl Plugin for SshServerPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<AsyncPlugin>(); }
}

/// Optional username/password credentials for an SSH server.
///
/// If set on [`SshServer`], only clients matching both fields are accepted.
/// If absent, all authentication attempts are accepted.
#[derive(Debug, Clone)]
pub struct SshCredentials {
	/// The required username.
	pub username: String,
	/// The required password.
	pub password: String,
}

impl SshCredentials {
	/// Creates new credentials from username and password.
	pub fn new(
		username: impl Into<String>,
		password: impl Into<String>,
	) -> Self {
		Self {
			username: username.into(),
			password: password.into(),
		}
	}
}

/// An SSH server that accepts incoming connections.
///
/// Each accepted connection spawns a child entity with [`SshPeerInfo`] and
/// bidirectional [`SshSend`]/[`SshRecv`] event flow.
///
/// Lifecycle events are delivered as [`SshRecv`] on the **connection entity**:
/// - [`SshEvent::Connect`] — a client opened a session
/// - [`SshEvent::Close`] — the client disconnected
///
/// The server owns the connection entity end to end: it is spawned on connect and
/// despawned once observers have seen the close, so per-connection state belongs
/// on it (and dies with it) rather than in a map a disconnect has to prune.
///
/// Register handlers as global observers, not per-server observers:
/// ```rust,ignore
/// app.add_observer(my_listener).world_mut().spawn(SshServer::default());
/// ```
#[derive(Clone, Component)]
#[component(on_add = on_add)]
pub struct SshServer {
	/// The port to bind to. `None` means the OS will assign a port.
	pub port: Option<u16>,
	/// The host address to bind to. Defaults to `[127, 0, 0, 1]` (localhost); use
	/// `[0, 0, 0, 0]` to listen on all interfaces (required for deployed servers).
	pub host: [u8; 4],
	/// Optional credentials. If `None`, all connections are accepted.
	pub credentials: Option<SshCredentials>,
}

impl std::fmt::Debug for SshServer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshServer")
			.field("port", &self.port)
			.field("host", &self.host)
			.field(
				"credentials",
				if self.credentials.is_some() {
					&"Some(..)"
				} else {
					&"None"
				},
			)
			.finish()
	}
}

#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	cfg_if! {
		if #[cfg(test)]{
			return;
		}
		else if #[cfg(all(feature = "russh_server", not(target_arch = "wasm32")))] {
			world
				.commands()
				.entity(cx.entity)
				.queue_async(super::impl_russh_server::start_russh_server);
		} else {
			panic!("SSH server requires the 'russh_server' feature on non-wasm32 targets");
		}
	}
}

impl SshServer {
	/// Creates a new SSH server bound to the specified port on localhost.
	pub fn new(port: u16) -> Self {
		Self {
			port: Some(port),
			host: [127, 0, 0, 1],
			credentials: None,
		}
	}

	/// Sets the host address to bind to (eg `[0, 0, 0, 0]` for all interfaces).
	pub fn with_host(mut self, host: [u8; 4]) -> Self {
		self.host = host;
		self
	}

	/// Sets the required credentials for this server.
	pub fn with_credentials(
		mut self,
		username: impl Into<String>,
		password: impl Into<String>,
	) -> Self {
		self.credentials = Some(SshCredentials::new(username, password));
		self
	}

	/// Creates a new server with an OS-assigned port for testing.
	///
	/// Binds to port 0 so the OS picks an available port,
	/// avoiding collisions in parallel tests. The listener is kept
	/// alive and passed directly to the server, eliminating port race conditions.
	///
	/// The `on_add` hook is disabled in tests, so the returned [`OnSpawn`]
	/// must be included in the spawn bundle to start the listener.
	#[cfg(all(feature = "russh_server", not(target_arch = "wasm32")))]
	pub fn new_test() -> (SshServer, OnSpawn) {
		let listener = std::net::TcpListener::bind("127.0.0.1:0")
			.expect("failed to bind test SSH server");
		let port = listener.local_addr().unwrap().port();
		(
			Self {
				port: Some(port),
				host: [127, 0, 0, 1],
				credentials: None,
			},
			OnSpawn::new_async(move |entity| {
				super::impl_russh_server::start_russh_server_with_tcp(
					entity, listener,
				)
			}),
		)
	}

	/// The client-facing host and port without the protocol, ie `127.0.0.1:2222`.
	pub fn local_address(&self) -> String {
		let port = self.port.unwrap_or(0);
		format!("127.0.0.1:{}", port)
	}

	/// The address the listener binds to, from [`host`](Self::host) and
	/// [`port`](Self::port), eg `0.0.0.0:2222` for a deployed server.
	pub fn bind_address(&self) -> String {
		let [a, b, c, d] = self.host;
		format!("{a}.{b}.{c}.{d}:{}", self.port.unwrap_or(0))
	}
}

impl Default for SshServer {
	/// Reads the process [`BootstrapConfig`] (`--ssh-port` / `BEET_SSH_PORT` and
	/// `--host` / `BEET_HOST`, a deployed server sets `BEET_HOST=0.0.0.0`),
	/// falling back to localhost on [`DEFAULT_SSH_PORT`].
	fn default() -> Self {
		let config = BootstrapConfig::get();
		Self {
			port: Some(config.ssh_port.unwrap_or(DEFAULT_SSH_PORT)),
			host: config.host_octets().unwrap_or([127, 0, 0, 1]),
			credentials: None,
		}
	}
}

#[cfg(test)]
#[cfg(all(
	feature = "russh_server",
	feature = "russh_client",
	not(target_arch = "wasm32")
))]
mod tests {
	use super::*;
	use crate::ssh::*;

	/// Poll `check` every 10ms until it holds, failing with `message` after ~2s.
	/// The server runs in its own thread, so its progress is only observable
	/// through the shared stores.
	async fn poll_until(mut check: impl FnMut() -> bool, message: &str) {
		for _ in 0..200 {
			if check() {
				return;
			}
			time_ext::sleep_millis(10).await;
		}
		panic!("{message}");
	}

	/// Connect anonymously and drop the session straight away, so the client is
	/// gone by the time this returns. Retries while the server thread boots, which
	/// a fixed sleep cannot promise on a loaded machine.
	async fn connect_then_disconnect(addr: &str) {
		for _ in 0..100 {
			if SshSession::connect_raw(addr, None, None).await.is_ok() {
				return;
			}
			time_ext::sleep_millis(20).await;
		}
		panic!("client never connected to the test server");
	}

	/// Regression: a gone client's connection entity goes with it, and it is
	/// announced before it is reclaimed. The accept loop spawns one entity per
	/// connection, so a server that never reclaims them keeps an entity — and the
	/// tokio task forwarding to that client — for every client that ever
	/// connected, which is a long-running server filling up.
	#[beet_core::test]
	async fn despawns_connection_on_disconnect() {
		let (server, on_spawn) = SshServer::new_test();
		let addr = server.local_address();
		let events = Store::<Vec<&'static str>>::default();
		let live = Store::<usize>::default();
		let (log, count) = (events.clone(), live.clone());

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin))
				.add_observer(move |ev: On<SshRecv>| match **ev {
					SshEvent::Connect => log.push("connect"),
					SshEvent::Close(_) => log.push("close"),
					_ => {}
				})
				.add_systems(Update, move |peers: Query<(), With<SshPeerInfo>>| {
					count.set(peers.iter().count())
				});
			app.world_mut().spawn((server, on_spawn));
			app.run();
		});

		connect_then_disconnect(&addr).await;

		// the whole lifecycle, in order: even a client gone this fast is announced
		// before it closes, and its entity is reclaimed after.
		poll_until(|| events.len() == 2, "server never saw the client go").await;
		events.get().xpect_eq(vec!["connect", "close"]);
		poll_until(|| live.get() == 0, "connection outlived its client").await;
	}

	/// Verifies that a client can connect and data flows bidirectionally.
	#[beet_core::test]
	async fn server_accepts_and_echoes() {
		let server = SshServer::new_test();
		let addr = server.0.local_address();
		let store = Store::<Option<String>>::default();
		let store_clone = store.clone();

		// start the bevy app with an echo server
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin))
				.add_observer(|ev: On<SshRecv>, mut commands: Commands| {
					if let Some(text) = ev.event().as_str() {
						commands.entity(ev.target()).trigger_target(SshSend(
							SshEvent::text(format!("echo:{}", text)),
						));
					}
				})
				.world_mut()
				.spawn(server);
			app.run();
		});

		// give the server time to start
		time_ext::sleep_millis(300).await;

		let store_inner = store_clone.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin::default()));
		app.world_mut()
			.spawn(SshSession::insert_on_connect(&addr, "guest", "beet"))
			.observe_any(|ev: On<SshRecv>, mut commands: Commands| {
				if matches!(**ev, SshEvent::Connect) {
					commands
						.entity(ev.target())
						.trigger_target(SshSend(SshEvent::text("hello")));
				}
			})
			.observe_any(move |ev: On<SshRecv>, mut commands: Commands| {
				if let Some(text) = ev.event().as_str() {
					store_inner.set(Some(text.to_owned()));
					commands.write_message(AppExit::Success);
				}
			});
		app.run();

		store.get().as_deref().xpect_eq(Some("echo:hello"));
	}

	/// Verifies that optional credentials are enforced.
	#[beet_core::test]
	async fn server_rejects_bad_credentials() {
		let (server, on_spawn) = SshServer::new_test();
		let server = server.with_credentials("admin", "secret");
		let addr = server.local_address();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin));
			app.world_mut().spawn((server, on_spawn));
			app.run();
		});

		time_ext::sleep_millis(300).await;

		// wrong password — should fail
		let result =
			SshSession::connect_raw(&addr, Some("admin"), Some("wrong")).await;
		result.xpect_err();
	}
}
