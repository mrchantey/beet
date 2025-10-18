//! WebSocket echo server example
//!
//! This example demonstrates how to create a WebSocket server that accepts
//! incoming connections and echoes messages back to clients.
//!
//! Run with:
//! ```sh
//! cargo run --example socket_server --features tungstenite,native-tls
//! ```
//!
//! Test with a WebSocket client:
//! ```sh
//! # In another terminal, run the echo endpoint test
//! cargo test --package beet_net --features tungstenite,native-tls socket::tests::echo_endpoint
//! ```

use beet::net::prelude::sockets::Message;
use beet::net::prelude::sockets::*;
use beet::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result {
	let addr = "127.0.0.1:9001";
	println!("Starting WebSocket server on ws://{}", addr);

	let mut server = SocketServer::bind(addr).await?;
	println!("Server listening, waiting for connections...");

	// Accept connections in a loop
	// Note: This handles one connection at a time synchronously
	// to avoid SendWrapper thread issues until sockets become truly Send
	while let Some(result) = server.next().await {
		match result {
			Ok(socket) => {
				println!("New client connected");
				// Handle connection synchronously on the same thread
				if let Err(e) = handle_connection(socket).await {
					eprintln!("Connection error: {}", e);
				}
			}
			Err(e) => {
				eprintln!("Failed to accept connection: {}", e);
			}
		}
	}

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
