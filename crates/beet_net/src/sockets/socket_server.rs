use crate::prelude::*;
use beet_core::prelude::*;
use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

/// A WebSocket server that can accept incoming connections.
///
/// Platform-specific implementations provide the actual binding and accept logic.
/// Each accepted connection returns a [`Socket`] that can be used like any client socket.
#[derive(Clone, Component)]
#[component(on_add = on_add)]
pub struct SocketServer {
	/// The address to bind to (e.g., "127.0.0.1:8080")
	pub port: u16,
}

impl std::fmt::Debug for SocketServer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SocketServer")
			.field("port", &self.port)
			.finish()
	}
}

#[allow(unused)]
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	world
		.commands()
		.run_system_cached_with(super::start_tungstenite_server, cx.entity);
	#[cfg(not(all(feature = "tungstenite", not(target_arch = "wasm32"))))]
	panic!(
		"WebSocket server requires the 'tungstenite' feature on non-wasm32 targets"
	);
}

impl SocketServer {
	pub fn new(port: u16) -> Self { Self { port } }

	/// Create a new Server with an incrementing port to avoid
	/// collisions in tests
	pub fn new_test() -> Self {
		static PORT: AtomicU16 = AtomicU16::new(DEFAULT_SOCKET_TEST_PORT);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			..default()
		}
	}

	/// The host and path without the protocol, ie `127.0.0.1:3000`
	pub fn local_address(&self) -> String { format!("127.0.0.1:{}", self.port) }
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

	#[sweet::test]
	async fn server_binds_and_accepts() {
		let server = SocketServer::new_test();
		let addr = server.local_address();

		// let _handle = tokio::spawn(async move {
		App::new()
			.add_plugins((
				MinimalPlugins,
				LogPlugin::default(),
				SocketServerPlugin::with_server(server),
			))
			.add_systems(PostStartup, move |mut commands: AsyncCommands| {
				let addr = addr.clone();
				commands.run(async move |world| {
					time_ext::sleep_millis(200).await;
					let url = format!("ws://{}", &addr);
					let mut client = Socket::connect(&url).await.unwrap();
					client.send(Message::text("hello server")).await.unwrap();
					client.close(None).await.ok();
					world.write_message(AppExit::Success);
				});
			})
			.run();
	}

	#[sweet::test]
	async fn handles_multiple_concurrent_connections() {
		let server = SocketServer::new_test();
		let addr = server.local_address();

		App::new()
			.add_plugins((
				MinimalPlugins,
				SocketServerPlugin::with_server(server),
			))
			.add_systems(PostStartup, move |mut commands: AsyncCommands| {
				let addr = addr.clone();
				commands.run(async move |world| {
					time_ext::sleep_millis(200).await;
					let url = format!("ws://{}", &addr);

					let mut client1 = Socket::connect(&url).await.unwrap();
					client1.send(Message::text("client1")).await.unwrap();

					time_ext::sleep_millis(100).await;

					let mut client2 = Socket::connect(&url).await.unwrap();
					client2.send(Message::text("client2")).await.unwrap();

					client1.close(None).await.ok();
					client2.close(None).await.ok();

					world.write_message(AppExit::Success);
				});
			})
			.run();
	}
}
