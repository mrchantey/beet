use std::sync::atomic::AtomicU16;
use std::sync::atomic::Ordering;

use beet_core::prelude::*;

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
		static PORT: AtomicU16 = AtomicU16::new(8340);
		Self {
			port: PORT.fetch_add(1, Ordering::SeqCst),
			..default()
		}
	}

	/// The host and path without the protocol, ie `127.0.0.1:3000`
	pub fn local_address(&self) -> String { format!("127.0.0.1:{}", self.port) }
}

impl Default for SocketServer {
	fn default() -> Self { Self::new(8080) }
}

#[cfg(test)]
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
mod tests {
	use super::*;
	use crate::sockets::Message;
	use crate::sockets::Socket;
	use crate::sockets::SocketServerPlugin;
	use crate::sockets::SocketServerStatus;

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
		println!("winner");
	}

	#[sweet::test]
	async fn handles_multiple_concurrent_connections() {
		let server = SocketServer::new_test();

		let (send, recv) = async_channel::bounded(1);
		let send_addr = send.clone();

		let _handle = tokio::spawn(async move {
			App::new()
				.add_plugins((
					MinimalPlugins,
					SocketServerPlugin::with_server(server),
				))
				.add_systems(
					Startup,
					move |query: Query<(Entity, &SocketServer)>,
					      mut commands: Commands| {
						let Ok((ent, _)) = query.single() else {
							return;
						};
						let send = send_addr.clone();
						commands.queue(move |world: &mut World| {
							// Wait a bit for server to start
							std::thread::sleep(
								std::time::Duration::from_millis(100),
							);
							if let Some(status) =
								world.entity(ent).get::<SocketServerStatus>()
							{
								send.try_send(status.local_addr()).ok();
							}
						});
					},
				)
				.run();
		});

		let addr = recv.recv().await.unwrap().unwrap();

		let client1_task = async {
			time_ext::sleep_millis(200).await;
			let url = format!("ws://{}", addr);
			let mut client = Socket::connect(&url).await.unwrap();
			client.send(Message::text("client1")).await.unwrap();
			time_ext::sleep_millis(500).await;
			client.close(None).await.ok();
		};

		let client2_task = async {
			time_ext::sleep_millis(300).await;
			let url = format!("ws://{}", addr);
			let mut client = Socket::connect(&url).await.unwrap();
			client.send(Message::text("client2")).await.unwrap();
			client.close(None).await.ok();
		};

		let _ = tokio::join!(client1_task, client2_task);
	}
}
