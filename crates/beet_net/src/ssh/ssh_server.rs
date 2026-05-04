use crate::ssh::*;
use beet_core::prelude::*;

/// Plugin for running bevy SSH servers.
pub struct SshServerPlugin {}

impl SshServerPlugin {}

impl Default for SshServerPlugin {
	fn default() -> Self { Self {} }
}

impl Plugin for SshServerPlugin {
	fn build(&self, app: &mut App) { app.init_plugin::<AsyncPlugin>(); }
}

/// An SSH server that accepts incoming connections.
///
/// Each accepted connection spawns a child entity with an [`SshConnection`]
/// component for bidirectional data exchange.
#[derive(Clone, Component)]
#[component(on_add = on_ssh_server_add)]
pub struct SshServer {
	/// The port to bind to. `None` means the OS will assign a port.
	pub port: Option<u16>,
}

impl std::fmt::Debug for SshServer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SshServer")
			.field("port", &self.port)
			.finish()
	}
}

#[allow(unused)]
fn on_ssh_server_add(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(test)]
	return;
	#[cfg(all(feature = "russh_server", not(target_arch = "wasm32")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async(super::impl_russh_server::start_russh_server);
	#[cfg(not(all(feature = "russh_server", not(target_arch = "wasm32"))))]
	panic!("SSH server requires 'russh_server' feature");
}

impl SshServer {
	/// Creates a new SSH server bound to the specified port.
	pub fn new(port: u16) -> Self { Self { port: Some(port) } }

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
			Self { port: Some(port) },
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

/// Bidirectional channel connecting the server to a single SSH client session.
#[derive(BundleEffect)]
pub struct SshConnection {
	pub(crate) to_client: async_channel::Sender<SshData>,
	pub(crate) from_client: async_channel::Receiver<SshData>,
}

impl SshConnection {
	fn effect(self, entity: &mut EntityWorldMut) {
		let to_client = self.to_client.clone();
		let from_client = self.from_client;

		entity
			.observe_any(
				move |ev: On<SshDataSend>,
				      mut commands: AsyncCommands|
				      -> Result {
					let to_client = to_client.clone();
					let data = ev.event().clone();
					commands.run_local(async move |_| {
						// forward data to client channel
						to_client
							.send(data.take())
							.await
							.unwrap_or_else(|err| error!("{:?}", err));
					});
					Ok(())
				},
			)
			.run_async_local(async move |entity| {
				while let Ok(data) = from_client.recv().await {
					entity.trigger_target_then(SshDataRecv(data)).await;
				}
				entity.trigger_target_then(SshClientDisconnected).await;
			});
	}
}

/// Triggered on the server entity when a new SSH client opens a session.
#[derive(EntityTargetEvent)]
pub struct SshClientConnected;

/// Triggered on a connection entity when the SSH client disconnects.
#[derive(EntityTargetEvent)]
pub struct SshClientDisconnected;

#[cfg(test)]
#[cfg(all(
	feature = "russh_server",
	feature = "russh_client",
	not(target_arch = "wasm32")
))]
mod tests {
	use super::*;

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
			app.add_plugins((MinimalPlugins, SshServerPlugin::default()));
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
		// use insert_on_connect so observers are set up before SshSessionReady fires
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
}
