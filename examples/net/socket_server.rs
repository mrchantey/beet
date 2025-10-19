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
use futures::StreamExt;

fn main() -> Result {
	App::new()
		.add_plugins((
			MinimalPlugins,
			LogPlugin::default(),
			SocketServerPlugin::with_server(SocketServer::new(9000)),
		))
		.run();

	Ok(())
}

async fn handle_connection(mut socket: Socket) -> Result<()> {
	println!("Handling new connection");

	// Echo messages back to the client
	while let Some(result) = socket.next().await {
		match result {
			Ok(msg) => match msg {
				Message::Text(text) => {
					println!("Received text: {}", text);
					socket.send(Message::text(text)).await?;
				}
				Message::Binary(data) => {
					println!("Received binary: {} bytes", data.len());
					socket.send(Message::binary(data)).await?;
				}
				Message::Ping(data) => {
					println!("Received ping");
					socket.send(Message::pong(data)).await?;
				}
				Message::Pong(_) => {
					println!("Received pong");
				}
				Message::Close(frame) => {
					println!(
						"Received close: {:?}",
						frame.as_ref().map(|f| &f.reason)
					);
					socket.close(frame).await?;
					break;
				}
			},
			Err(e) => {
				eprintln!("Error receiving message: {}", e);
				break;
			}
		}
	}

	println!("Connection closed");
	Ok(())
}
