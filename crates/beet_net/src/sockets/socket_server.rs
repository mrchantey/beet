use crate::prelude::*;
use beet_core::prelude::*;

/// Plugin for running bevy WebSocket servers.
pub struct SocketServerPlugin {}

impl SocketServerPlugin {}

impl Default for SocketServerPlugin {
	fn default() -> Self { Self {} }
}

impl Plugin for SocketServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		// we may want to add more later
	}
}


/// A WebSocket server that can accept incoming connections.
///
/// Platform-specific implementations provide the actual binding and accept logic.
/// Each accepted connection returns a [`Socket`] that can be used like any client socket.
#[derive(Clone, Component)]
#[component(on_add = on_add)]
pub struct SocketServer {
	/// The port to bind to. `None` means the OS will assign a port.
	pub port: Option<u16>,
}

impl std::fmt::Debug for SocketServer {
	fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		fmt.debug_struct("SocketServer")
			.field("port", &self.port)
			.finish()
	}
}

#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(test)]
	return;
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	world
		.commands()
		.entity(cx.entity)
		.queue_async(super::start_tungstenite_server);
	#[cfg(not(all(feature = "tungstenite", not(target_arch = "wasm32"))))]
	panic!(
		"WebSocket server requires the 'tungstenite' feature on non-wasm32 targets"
	);
}

impl SocketServer {
	/// Creates a new socket server bound to the specified port.
	pub fn new(port: u16) -> Self { Self { port: Some(port) } }

	/// Creates a new server with an OS-assigned port for testing.
	///
	/// Binds to port 0 so the OS picks an available port,
	/// avoiding collisions in parallel tests.
	///
	/// The `on_add` hook is disabled in tests, so the returned [`OnSpawn`]
	/// must be included in the spawn bundle to start the listener.
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	pub async fn new_test() -> (SocketServer, OnSpawn) {
		// Bind to port 0 to get an OS-assigned port
		let listener = std::net::TcpListener::bind("127.0.0.1:0")
			.expect("failed to bind test socket server");
		let port = listener.local_addr().unwrap().port();
		drop(listener);
		(
			Self { port: Some(port) },
			OnSpawn::new_async(super::start_tungstenite_server),
		)
	}

	/// The host and port without the protocol, ie `127.0.0.1:3000`
	pub fn local_address(&self) -> String {
		let port = self.port.unwrap_or(0);
		format!("127.0.0.1:{}", port)
	}
	/// Returns the full WebSocket URL for local connections, e.g. `ws://127.0.0.1:8339`.
	pub fn local_url(&self) -> String {
		format!("ws://{}", self.local_address())
	}
}



impl Default for SocketServer {
	fn default() -> Self { Self::new(DEFAULT_SOCKET_PORT) }
}

#[cfg(test)]
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
mod tests {
	use super::*;
	use crate::sockets::Message;
	use crate::sockets::*;

	#[beet_core::test]
	async fn server_binds_and_accepts() {
		let server = SocketServer::new_test().await;
		let url = server.0.local_url();

		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, SocketServerPlugin::default()))
				.spawn_then(server)
				.run();
		});
		time_ext::sleep_millis(200).await;

		let mut client = Socket::connect(&url).await.unwrap();
		client.send(Message::text("hello server")).await.unwrap();
		client.close(None).await.ok();
	}

	#[beet_core::test]
	async fn handles_multiple_concurrent_connections() {
		let server = SocketServer::new_test().await;
		let url = server.0.local_url();

		std::thread::spawn(move || {
			App::new()
				.add_plugins((MinimalPlugins, SocketServerPlugin::default()))
				.spawn_then(server)
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
		let server = SocketServer::new_test().await;
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
