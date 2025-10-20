//! WebSocket client example
//!
//! This example demonstrates how to connect to a WebSocket server and
//! send/receive messages.
//!
//! Make sure the server is running first:
//! ```sh
//! cargo run --example socket_server --features tungstenite
//! ```
//!
//! Run with:
//! ```sh
//! cargo run --example socket_client --features tungstenite
//! ```
//!

use beet::net::prelude::sockets::Message;
use beet::net::prelude::sockets::*;
use beet::prelude::*;
use beet_net::sockets::common_handlers::PingTime;

fn main() {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			AsyncPlugin::default(),
		))
		.spawn_then((
			Socket::insert_on_connect("ws://127.0.0.1:9000"),
			OnSpawn::observe(on_ready),
			OnSpawn::observe(my_handler),
			OnSpawn::observe(common_handlers::log_send),
			OnSpawn::observe(common_handlers::echo_pingtime),
			OnSpawn::observe(common_handlers::log_recv),
		))
		.run();
	info!("Done");
}


fn on_ready(ev: On<SocketReady>, mut commands: Commands) {
	commands
		.entity(ev.target())
		// we'll send a PingTime ping to get the RTT
		.trigger_target(MessageSend(PingTime::default().into_message()))
		// we'll also send a text message that the server is expecting
		.trigger_target(MessageSend(Message::Text(
			"the cat sat on the".into(),
		)));
}

fn my_handler(ev: On<MessageRecv>, mut commands: Commands) {
	match ev.event().inner() {
		// on receiving the expected message we'll send a close message
		Message::Text(txt) if txt == "hat" => {
			commands
				.entity(ev.target())
				.trigger_target(MessageSend(Message::Close(None)));
		}
		// exit the app when the close is acknowledged
		Message::Close(_) => {
			println!("done!");
			commands.write_message(AppExit::Success);
		}
		_ => {}
	}
}
