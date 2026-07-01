//! A LAN-reachable WebSocket echo server, for testing a remote client (eg an esp
//! device) against beet's `Socket` API. Binds `0.0.0.0` on the `SOCKET_SERVER`
//! port (default `1111`) and echoes every text/binary message back, logging the
//! traffic so the device's round-trip is visible in the host terminal.
//!
//! This is the server behind the `beet socket-server` verb (see
//! `beet_esp/main.bsx`); unlike `socket_server.rs` (a loopback demo with a fixed
//! reply), it binds all interfaces and echoes generically.
//!
//! Run with: `cargo run --example socket_echo_server --features tungstenite,action`

use beet::prelude::*;
use beet::prelude::sockets::*;

fn main() -> Result {
	let port = socket_server_port();
	info!("beet socket-server: binding ws://0.0.0.0:{port} (echo)");
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			SocketServerPlugin::default(),
		))
		.add_systems(Startup, move |mut commands: Commands| {
			commands
				.spawn((
					SocketServer::new(port).bind_all(),
					OnSpawn::observe(common_handlers::echo_message),
					OnSpawn::observe(common_handlers::echo_close),
					OnSpawn::observe(common_handlers::log_send),
					OnSpawn::observe(common_handlers::log_recv),
				))
				.trigger(StartRunning::boot);
		})
		.run();

	Ok(())
}

/// The port from the `SOCKET_SERVER` (`host:port`) env var, else `1111`.
fn socket_server_port() -> u16 {
	env_ext::var("SOCKET_SERVER")
		.ok()
		.and_then(|addr| {
			addr.rsplit_once(':').and_then(|(_, port)| port.parse().ok())
		})
		.unwrap_or(1111)
}
