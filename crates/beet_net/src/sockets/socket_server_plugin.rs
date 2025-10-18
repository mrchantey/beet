use crate::sockets::*;
use beet_core::prelude::*;

/// Plugin for running bevy WebSocket servers.
/// By default this plugin will spawn the default [`SocketServer`] on [`Startup`]
pub struct SocketServerPlugin {
	/// Spawn the server on add
	pub spawn_server: Option<SocketServer>,
}

impl SocketServerPlugin {
	/// Create a new SocketServerPlugin that does not spawn a server
	pub fn without_server(mut self) -> Self {
		self.spawn_server = None;
		self
	}

	pub fn with_server(server: SocketServer) -> Self {
		Self {
			spawn_server: Some(server),
			..default()
		}
	}
}

impl Default for SocketServerPlugin {
	fn default() -> Self {
		Self {
			spawn_server: Some(SocketServer::default()),
		}
	}
}

impl Plugin for SocketServerPlugin {
	fn build(&self, app: &mut App) {
		app.init_plugin::<AsyncPlugin>();
		if let Some(server) = &self.spawn_server {
			let server = server.clone();
			app.add_systems(Startup, move |mut commands: Commands| {
				commands.spawn(server.clone());
			});
		}
	}
}
