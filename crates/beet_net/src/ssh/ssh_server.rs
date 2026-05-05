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
/// bidirectional [`SshDataSend`]/[`SshDataRecv`] event flow.
#[derive(Clone, Component)]
#[component(on_add = on_add)]
pub struct SshServer {
	/// The port to bind to. `None` means the OS will assign a port.
	pub port: Option<u16>,
	/// Optional credentials. If `None`, all connections are accepted.
	pub credentials: Option<SshCredentials>,
}

impl std::fmt::Debug for SshServer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshServer")
			.field("port", &self.port)
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
	/// Creates a new SSH server bound to the specified port.
	pub fn new(port: u16) -> Self {
		Self {
			port: Some(port),
			credentials: None,
		}
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
				credentials: None,
			},
			OnSpawn::new_async(move |entity| {
				super::impl_russh_server::start_russh_server_with_tcp(
					entity, listener,
				)
			}),
		)
	}

	/// The host and port without the protocol, ie `127.0.0.1:2222`.
	pub fn local_address(&self) -> String {
		let port = self.port.unwrap_or(0);
		format!("127.0.0.1:{}", port)
	}
}

impl Default for SshServer {
	fn default() -> Self { Self::new(2222) }
}

/// Triggered when a new SSH client opens a session.
///
/// Propagates from the connection (child) entity up to the server entity,
/// so observers on the server receive it with `ev.original_target()` pointing
/// to the connection entity.
#[derive(EntityTargetEvent)]
#[event(auto_propagate)]
pub struct SshClientConnected;

/// Triggered when the SSH client disconnects.
///
/// Propagates from the connection (child) entity up to the server entity,
/// so observers on the server receive it with `ev.original_target()` pointing
/// to the connection entity.
#[derive(EntityTargetEvent)]
#[event(auto_propagate)]
pub struct SshClientDisconnected;

#[cfg(test)]
#[cfg(all(
	feature = "russh_server",
	feature = "russh_client",
	not(target_arch = "wasm32")
))]
mod tests {
	use super::*;
	use crate::ssh::*;

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
			app.add_plugins((MinimalPlugins, SshServerPlugin));
			app.world_mut().spawn(server).observe_any(
				|ev: On<SshDataRecv>, mut commands: Commands| {
					if let Some(text) = ev.event().inner().as_str() {
						commands.entity(ev.original_target()).trigger_target(
							SshDataSend(SshData::text(format!(
								"echo:{}",
								text
							))),
						);
					}
				},
			);
			app.run();
		});

		// give the server time to start
		time_ext::sleep_millis(300).await;

		let store_inner = store_clone.clone();
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin::default()));
		app.world_mut()
			.spawn(SshSession::insert_on_connect(&addr, "guest", "beet"))
			.observe_any(|ev: On<SshSessionReady>, mut commands: Commands| {
				commands
					.entity(ev.target())
					.trigger_target(SshDataSend(SshData::text("hello")));
			})
			.observe_any(move |ev: On<SshDataRecv>, mut commands: Commands| {
				if let Some(text) = ev.event().inner().as_str() {
					store_inner.set(Some(text.to_owned()));
				}
				commands.write_message(AppExit::Success);
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
