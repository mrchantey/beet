use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin for running bevy SSH servers.
///
/// Beyond the async runtime the listener needs, this drives
/// [`SshServer::pty_timeout`]: without it a server still accepts connections, but
/// nothing enforces the terminal request.
#[derive(Default)]
pub struct SshServerPlugin;

impl Plugin for SshServerPlugin {
	fn build(&self, app: &mut App) {
		app.install_type::<AsyncPlugin>()
			.register_type::<SshHostKeyRef>()
			.register_type::<SshHostKeyConsumers>()
			.add_observer(RequirePty::clear_on_pty)
			.add_systems(Update, RequirePty::close_expired);
	}
}

/// Closes its connection unless the client requests a pty before the timer
/// elapses.
///
/// Inserted on each connection when the server sets a
/// [`pty_timeout`](SshServer::pty_timeout), and removed the moment the pty
/// request arrives, so it only ever fires on a client that opened a channel and
/// then asked for nothing: the shape of the credential scanners that find any
/// public port 22 within hours. A real client requests its terminal immediately.
#[derive(Debug, Clone, Component)]
pub struct RequirePty(Timer);

impl RequirePty {
	/// Require a pty request within `timeout` of the connection opening.
	pub fn new(timeout: Duration) -> Self {
		Self(Timer::new(timeout, TimerMode::Once))
	}

	/// Observer: the client asked for its terminal, so the requirement is met.
	fn clear_on_pty(ev: On<SshRecv>, mut commands: Commands) {
		if matches!(**ev, SshEvent::RequestPty(_)) {
			commands.entity(ev.target()).try_remove::<Self>();
		}
	}

	/// System: close every connection whose pty deadline passed. The close is
	/// announced to the client and disconnects the session, so a peer that ignores
	/// it still loses the socket.
	fn close_expired(
		time: Res<Time>,
		mut commands: Commands,
		mut connections: Populated<(Entity, &mut Self)>,
	) {
		for (entity, mut require) in connections.iter_mut() {
			if require.0.tick(time.delta()).is_finished() {
				debug!("closing ssh connection {entity}: no pty requested");
				// the connection may already be gone: its recv loop despawns it the
				// moment the client drops, which is the common end for the very
				// scanners this reaps.
				commands
					.entity(entity)
					.try_remove::<Self>()
					.try_trigger_target(SshSend(SshEvent::Close(None)));
			}
		}
	}
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
	/// How long a connection may go without traffic before the transport drops
	/// it, [`DEFAULT_IDLE_TIMEOUT`] by default.
	///
	/// The only reaper for a peer that authenticates and then just holds the
	/// socket, so an hour-long value means an hour-long slot per stalled scan.
	/// Its floor is how long a *real* session may sit unread: a terminal reading
	/// a page sends nothing, so this is the price of not dropping them.
	pub idle_timeout: Duration,
	/// How long a connection has to request a pty before it is closed, `None`
	/// (the default) to let a connection live without one.
	///
	/// Set it on a server whose sessions are terminals: it costs a real client
	/// nothing (its pty request follows auth immediately) and takes the scanners
	/// out in seconds rather than at [`idle_timeout`](Self::idle_timeout). Leave
	/// it unset for a server serving non-terminal clients, eg beet's own
	/// [`SshSession`](crate::prelude::SshSession) data channels.
	///
	/// Enforced by [`SshServerPlugin`] (via [`RequirePty`]), not by the transport.
	pub pty_timeout: Option<Duration>,
	/// The key the server presents, its fingerprint the one clients pin. When
	/// `None` the listener reads the [`SshHostKeyRef`] on its entity, else the
	/// process default (see [`SshHostKey`]).
	pub host_key: Option<SshHostKey>,
}

/// The default [`SshServer::idle_timeout`]: long enough to read a page without
/// being dropped mid-session, short enough that a stalled scan is not a slot held
/// for an hour.
pub const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

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
			.field("idle_timeout", &self.idle_timeout)
			.field("pty_timeout", &self.pty_timeout)
			.field("host_key", &self.host_key)
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
		} else if #[cfg(target_arch = "wasm32")] {
			// no ssh transport exists in a wasm sandbox, so the component stays
			// and the listener does not: a document declaring one loads whole
			// and only its behavior is missing.
			let _ = cx;
			warn!(
				"SshServer is declared but wasm32 has no ssh transport, so no listener is started"
			);
		} else {
			world.commands().handle_command_error::<SshServer>(bevyhow!(
				"SshServer requires the `russh_server` feature"
			));
		}
	}
}

impl SshServer {
	/// Creates a new SSH server bound to the specified port on localhost.
	pub fn new(port: u16) -> Self { Self::bound([127, 0, 0, 1], Some(port)) }

	/// An open server on `host:port`: no credentials, default timeouts. The one
	/// place the field defaults live, so a constructor cannot drift from them.
	fn bound(host: [u8; 4], port: Option<u16>) -> Self {
		Self {
			port,
			host,
			credentials: None,
			idle_timeout: DEFAULT_IDLE_TIMEOUT,
			pty_timeout: None,
			host_key: None,
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

	/// Sets how long a connection may go without traffic, see
	/// [`idle_timeout`](Self::idle_timeout).
	pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
		self.idle_timeout = timeout;
		self
	}

	/// Requires each connection to request a pty within `timeout`, see
	/// [`pty_timeout`](Self::pty_timeout).
	pub fn with_pty_timeout(mut self, timeout: Duration) -> Self {
		self.pty_timeout = Some(timeout);
		self
	}

	/// Presents `key` rather than the process default, see
	/// [`host_key`](Self::host_key).
	pub fn with_host_key(mut self, key: SshHostKey) -> Self {
		self.host_key = Some(key);
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
			Self::bound([127, 0, 0, 1], Some(port)),
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

/// The ed25519 key an [`SshServer`] presents, held in the encoding it arrived
/// in: the file `ssh-keygen` writes (its PEM armor), base64 of that file
/// (wrapped or not), or base64 of the binary encoding inside it. [`Debug`]
/// redacts.
///
/// A server with no declared key reads the process default: the launch's
/// `BEET_SSH_HOST_KEY`, else in a debug build a fixed key so a restart keeps
/// its fingerprint, else a random one. A deployed server declares its key
/// instead ([`SshHostKeyRef`]), and a declared key that does not parse fails
/// the server rather than serving under a stranger's fingerprint.
#[derive(Clone)]
pub struct SshHostKey(String);

impl std::fmt::Debug for SshHostKey {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("SshHostKey(..)")
	}
}

impl SshHostKey {
	/// The fixed key a debug build presents when none is declared, so a local
	/// restart keeps its fingerprint, as base64 of its binary encoding. Public
	/// by design, so never a deployed key: a test needing a key that parses
	/// under every feature set stores this one.
	pub const DEBUG: &'static str = "b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZWQyNTUxOQAAACBUG35nYe8356vXPMw3zqK4+Q9H1/XX4UuCrbu2tQrSZAAAAIjdqOuF3ajrhQAAAAtzc2gtZWQyNTUxOQAAACBUG35nYe8356vXPMw3zqK4+Q9H1/XX4UuCrbu2tQrSZAAAAEDSBMO7QAGg41Elgt3IFRQGvS69bvIuQ7eNMcDGmcM9K1Qbfmdh7zfnq9c8zDfOorj5D0fX9dfhS4Ktu7a1CtJkAAAAAAECAwQF";

	/// `encoded` as a host key, checked to parse wherever this build can serve
	/// ssh, so a malformed key fails whoever declared it rather than the
	/// listener later.
	pub fn parse(encoded: impl Into<String>) -> Result<Self> {
		let key = Self(encoded.into());
		#[cfg(all(feature = "russh_server", not(target_arch = "wasm32")))]
		super::impl_russh_server::parse_host_key(&key.0)?;
		key.xok()
	}

	/// The key as given, for the listener to parse.
	#[allow(unused)]
	pub(crate) fn encoded(&self) -> &str { &self.0 }

	/// The key `entity` declares through its [`SshHostKeyRef`], `None` when it
	/// declares none or its [`StoredSecret`] holds none and allows that (the
	/// process default then applies). A secret that cannot be read (a
	/// deployed key missing from its stack's store, a store that refuses)
	/// is an error naming it, never a fall back to a key nobody pinned.
	pub async fn resolve(entity: &AsyncEntity) -> Result<Option<Self>> {
		let Ok(target) = entity
			.get::<SshHostKeyRef, _>(|key_ref| key_ref.source())
			.await
		else {
			return Ok(None);
		};
		let secret = StoredSecret::resolve(entity.world(), target).await?;
		let Some(value) = secret.get().await.map_err(|err| {
			bevyhow!("ssh host key from {}: {err}", secret.describe())
		})?
		else {
			return Ok(None);
		};
		let key = Self::parse(value).map_err(|err| {
			bevyhow!("malformed ssh host key from {}: {err}", secret.describe())
		})?;
		info!("ssh host key from {}", secret.describe());
		Some(key).xok()
	}
}

/// The secret an ssh server reads its [`SshHostKey`] from: the source half of
/// the [`SshHostKeyConsumers`] relationship, on the server's entity,
/// targeting a declaration that lands a [`StoredSecret`]. Authored as
/// `{SshHostKeyRef($ssh_host_key)}` beside a `<SecretBlock
/// bx:ref="ssh_host_key" ../>`, so a local launch reads the record its
/// `secrets.toml` loads and a deployed box reads its stack's secret store.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
#[relationship(relationship_target = SshHostKeyConsumers)]
pub struct SshHostKeyRef(#[entities] pub Entity);

impl SshHostKeyRef {
	/// The declaration this server reads its key from.
	pub fn source(&self) -> Entity { self.0 }
}

/// Every ssh server reading its key from a declaration: the target half of
/// the [`SshHostKeyRef`] relationship.
#[derive(Debug, Default, Reflect, Component)]
#[reflect(Component)]
#[relationship_target(relationship = SshHostKeyRef)]
pub struct SshHostKeyConsumers(Vec<Entity>);

impl Default for SshServer {
	/// Reads the process [`BootstrapConfig`] (`--ssh-port` / `BEET_SSH_PORT` and
	/// `--host` / `BEET_HOST`, a deployed server sets `BEET_HOST=0.0.0.0`),
	/// falling back to localhost on [`DEFAULT_SSH_PORT`].
	fn default() -> Self {
		let config = BootstrapConfig::get();
		Self::bound(
			config.host_octets().unwrap_or([127, 0, 0, 1]),
			Some(config.ssh_port.unwrap_or(DEFAULT_SSH_PORT)),
		)
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
		poll_ext::poll_with(
			|| {
				match check() {
					true => ControlFlow::Break(()),
					false => ControlFlow::Continue(()),
				}
				.xok()
			},
			Duration::from_secs(2),
			Duration::from_millis(10),
		)
		.await
		.unwrap_or_else(|_| panic!("{message}"));
	}

	/// Connect anonymously and drop the session straight away, so the client is
	/// gone by the time this returns. Retries while the server thread boots, which
	/// a fixed sleep cannot promise on a loaded machine.
	async fn connect_then_disconnect(addr: &str) {
		poll_ext::poll_result_async_with(
			async || SshSession::connect_raw(addr, None, None).await.map(drop),
			Duration::from_secs(2),
			Duration::from_millis(20),
		)
		.await
		.expect("client never connected to the test server");
	}

	/// A client that opens a channel and then asks for nothing, the port-scanner
	/// shape, is closed by the server once its pty timeout passes.
	///
	/// Asserted on the *server*: the client here sits on its session and would
	/// otherwise be dropped only by a timeout an order of magnitude further out,
	/// so a connection reclaimed within the poll budget can only be the reap.
	#[beet_core::test]
	async fn reaps_a_connection_that_never_requests_a_pty() {
		let (server, on_spawn) = SshServer::new_test();
		let server = server.with_pty_timeout(Duration::from_millis(200));
		let addr = server.local_address();
		let live = Store::<usize>::default();
		let count = live.clone();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin))
				.add_systems(
					Update,
					move |peers: Query<(), With<SshPeerInfo>>| {
						count.set(peers.iter().count())
					},
				);
			app.world_mut().spawn((server, on_spawn));
			app.run();
		});
		time_ext::sleep_millis(300).await;

		// a client that connects and sits there: no pty, no shell, no data
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AsyncPlugin::default()));
			app.world_mut().spawn(SshSession::insert_anon(&addr));
			app.run();
		});

		poll_until(|| live.get() == 1, "server never saw the connection").await;
		poll_until(|| live.get() == 0, "connection with no pty was not reaped")
			.await;
	}

	/// A session that does request a pty is left alone: the requirement is met,
	/// so the timer that would have closed it is gone.
	#[beet_core::test]
	async fn a_pty_request_clears_the_requirement() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, SshServerPlugin));
		let connection = app
			.world_mut()
			.spawn(RequirePty::new(Duration::from_millis(50)))
			.flush();

		app.world_mut()
			.entity_mut(connection)
			.trigger_target(SshRecv(SshEvent::RequestPty(RequestPty {
				terminal: "xterm".into(),
				..default()
			})));
		app.update();

		app.world()
			.entity(connection)
			.contains::<RequirePty>()
			.xpect_false();
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
				.add_systems(
					Update,
					move |peers: Query<(), With<SshPeerInfo>>| {
						count.set(peers.iter().count())
					},
				);
			app.world_mut().spawn((server, on_spawn));
			app.run();
		});

		connect_then_disconnect(&addr).await;

		// the whole lifecycle, in order: even a client gone this fast is announced
		// before it closes, and its entity is reclaimed after.
		poll_until(|| events.len() == 2, "server never saw the client go")
			.await;
		events.get().xpect_eq(vec!["connect", "close"]);
		poll_until(|| live.get() == 0, "connection outlived its client").await;
	}

	/// Serve a test server that answers each event its client sends with the
	/// events `reply` returns, returning its address once it is up.
	async fn serve_replies(reply: fn(&SshEvent) -> Vec<SshEvent>) -> String {
		let server = SshServer::new_test();
		let addr = server.0.local_address();
		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SshServerPlugin))
				.add_observer(move |ev: On<SshRecv>, mut commands: Commands| {
					for answer in reply(&ev) {
						commands
							.entity(ev.target())
							.trigger_target(SshSend(answer));
					}
				})
				.world_mut()
				.spawn(server);
			app.run();
		});
		// give the server time to start
		time_ext::sleep_millis(300).await;
		addr
	}

	/// [`serve_replies`], then have a client send `first` once connected and
	/// return the first output it gets back, on either stream.
	async fn first_reply(
		reply: fn(&SshEvent) -> Vec<SshEvent>,
		first: SshEvent,
	) -> Option<SshEvent> {
		let addr = serve_replies(reply).await;
		let store = Store::<Option<SshEvent>>::default();
		let store_inner = store.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin::default()));
		app.world_mut()
			.spawn(SshSession::insert_on_connect(&addr, "guest", "beet"))
			.observe_any(move |ev: On<SshRecv>, mut commands: Commands| {
				if matches!(**ev, SshEvent::Connect) {
					commands
						.entity(ev.target())
						.trigger_target(SshSend(first.clone()));
				}
			})
			.observe_any(move |ev: On<SshRecv>, mut commands: Commands| {
				if matches!(**ev, SshEvent::Data(_) | SshEvent::Stderr(_)) {
					store_inner.set(Some(ev.event().inner().clone()));
					commands.write_message(AppExit::Success);
				}
			});
		app.run();
		store.get()
	}

	/// Verifies that a client can connect and data flows bidirectionally.
	#[beet_core::test]
	async fn server_accepts_and_echoes() {
		first_reply(
			|ev| {
				ev.as_str()
					.map(|text| SshEvent::text(format!("echo:{text}")))
					.into_iter()
					.collect()
			},
			SshEvent::text("hello"),
		)
		.await
		.unwrap()
		.as_str()
		.xpect_eq(Some("echo:hello"));
	}

	/// An exec request carries its command across the transport, ie the
	/// `/docs` of `ssh -t <host> /docs`.
	#[beet_core::test]
	async fn server_receives_the_exec_command() {
		first_reply(
			|ev| match ev {
				SshEvent::RequestExec(command) => {
					vec![SshEvent::text(format!("exec:{command}"))]
				}
				_ => vec![],
			},
			SshEvent::RequestExec("/docs".into()),
		)
		.await
		.unwrap()
		.as_str()
		.xpect_eq(Some("exec:/docs"));
	}

	/// A diagnostic reaches the client's stderr as SSH extended data, apart
	/// from the stdout a pipe captures as the answer.
	#[beet_core::test]
	async fn stderr_reaches_the_client_apart_from_stdout() {
		let reply = first_reply(
			|ev| match ev {
				SshEvent::RequestExec(_) => vec![SshEvent::stderr("oops")],
				_ => vec![],
			},
			SshEvent::RequestExec("/docs".into()),
		)
		.await;
		matches!(reply, Some(SshEvent::Stderr(bytes)) if bytes.as_ref() == b"oops")
			.xpect_true();
	}

	/// A russh client handler recording whether the server sent a disconnect.
	struct DisconnectRecorder(Store<bool>);

	impl russh::client::Handler for DisconnectRecorder {
		type Error = russh::Error;

		async fn check_server_key(
			&mut self,
			_key: &russh::keys::ssh_key::PublicKey,
		) -> std::result::Result<bool, Self::Error> {
			Ok(true)
		}

		async fn disconnected(
			&mut self,
			reason: russh::client::DisconnectReason<Self::Error>,
		) -> std::result::Result<(), Self::Error> {
			match reason {
				russh::client::DisconnectReason::ReceivedDisconnect(_) => {
					self.0.set(true);
					Ok(())
				}
				russh::client::DisconnectReason::Error(err) => Err(err),
			}
		}
	}

	/// Regression: a session the server closes leaves the hang-up to its client.
	/// A disconnect is fatal to OpenSSH, which exits 255 and drops output it has
	/// not yet written, so disconnecting straight after the close lost both the
	/// hint a pty-less `ssh <host> /blog` is sent and its exit code. This client
	/// answers the close, as OpenSSH does, then holds the connection past it, so
	/// a premature disconnect would reach it.
	#[beet_core::test]
	async fn a_closed_session_leaves_the_hang_up_to_its_client() {
		let addr = serve_replies(|ev| match ev {
			SshEvent::RequestExec(_) => vec![
				SshEvent::text("bye"),
				SshEvent::Close(Some(SshCloseFrame {
					code: 3,
					reason: default(),
				})),
			],
			_ => vec![],
		})
		.await;
		let disconnected = Store::<bool>::default();
		let recorder = DisconnectRecorder(disconnected.clone());

		let (output, exit) = async_ext::on_tokio(async move {
			let config = std::sync::Arc::new(russh::client::Config::default());
			let mut session =
				russh::client::connect(config, addr, recorder).await?;
			session.authenticate_none("guest").await?;
			let mut channel = session.channel_open_session().await?;
			channel.exec(true, "/blog").await?;
			let (mut output, mut exit) = (String::new(), None);
			while let Some(msg) = channel.wait().await {
				match msg {
					russh::ChannelMsg::Data { data } => {
						output.push_str(&String::from_utf8_lossy(&data));
					}
					russh::ChannelMsg::ExitStatus { exit_status } => {
						exit = Some(exit_status);
					}
					russh::ChannelMsg::Close => break,
					_ => {}
				}
			}
			// a client still draining its output holds the connection open
			time_ext::sleep_millis(500).await;
			(output, exit).xok()
		})
		.await
		.unwrap();

		output.xpect_eq("bye");
		exit.xpect_eq(Some(3));
		disconnected.get().xpect_false();
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
