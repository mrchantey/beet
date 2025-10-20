//! WebSocket echo server example
//!
//! This example demonstrates how to create a WebSocket server that accepts
//! incoming connections and echoes messages back to clients.
//!
//! Run with:
//! ```sh
//! cargo run --example socket_server --features tungstenite
//! ```
//! Test with a WebSocket client:
//! ```sh
//! # In another terminal, run the echo endpoint test
//! cargo run --example socket_client --features tungstenite
//! ```

use beet::net::prelude::sockets::Message;
use beet::net::prelude::sockets::*;
use beet::prelude::*;

fn main() -> Result {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			SocketServerPlugin::default(),
		))
		.spawn_then((
			SocketServer::new(9000),
			OnSpawn::observe(my_handler),
			// just echo back the close if you dont need to
			// modify the CloseFrame
			OnSpawn::observe(common_handlers::echo_close),
			OnSpawn::observe(common_handlers::log_send),
			OnSpawn::observe(common_handlers::log_recv),
		))
		.run();

	Ok(())
}

// an example handler receiving and sending messages.
fn my_handler(recv: On<MessageRecv>, mut commands: Commands) {
	match recv.event().inner() {
		Message::Text(txt) if txt == "the cat sat on the" => {
			commands
				.entity(
					// we added this observer to the server, the message was
					// triggered by its child so we must use original_target
					recv.original_target(),
				)
				.trigger_target(MessageSend(Message::Text("hat".into())));
		}
		_ => {}
	}
}
