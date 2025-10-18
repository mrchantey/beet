//! WebSocket client example
//!
//! This example demonstrates how to connect to a WebSocket server and
//! send/receive messages.
//!
//! Run with:
//! ```sh
//! cargo run --example socket_client --features tungstenite,native-tls
//! ```
//!
//! Make sure the server is running first:
//! ```sh
//! cargo run --example socket_server --features tungstenite,native-tls
//! ```

use beet::net::prelude::sockets::Message;
use beet::net::prelude::sockets::*;
use beet::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result {
	let url = "ws://127.0.0.1:9001";
	println!("Connecting to WebSocket server at {}", url);

	let mut socket = Socket::connect(url).await?;
	println!("Connected to server!");

	// Send some test messages
	let messages = vec![
		"Hello, server!",
		"This is a test message",
		"How are you doing?",
		"Goodbye!",
	];

	for msg in messages {
		println!("Sending: {}", msg);
		socket.send(Message::text(msg)).await?;

		// Wait for echo response
		if let Some(result) = socket.next().await {
			match result {
				Ok(Message::Text(text)) => {
					println!("Received echo: {}", text);
				}
				Ok(Message::Binary(data)) => {
					println!("Received binary echo: {} bytes", data.len());
				}
				Ok(Message::Close(frame)) => {
					println!(
						"Server closed connection: {:?}",
						frame.as_ref().map(|f| &f.reason)
					);
					break;
				}
				Ok(_) => {
					println!("Received other message type");
				}
				Err(e) => {
					eprintln!("Error receiving message: {}", e);
					break;
				}
			}
		}

		// Small delay between messages
		time_ext::sleep_millis(100).await;
	}

	// Send a binary message
	println!("Sending binary message");
	let binary_data = vec![1u8, 2, 3, 4, 5];
	socket.send(Message::binary(binary_data.clone())).await?;

	if let Some(result) = socket.next().await {
		match result {
			Ok(Message::Binary(data)) => {
				println!("Received binary echo: {:?}", data.as_ref());
			}
			Ok(msg) => {
				println!("Received unexpected message type: {:?}", msg);
			}
			Err(e) => {
				eprintln!("Error: {}", e);
			}
		}
	}

	// Close the connection gracefully
	println!("Closing connection");
	socket
		.close(Some(CloseFrame {
			code: 1000,
			reason: "Client finished".to_string(),
		}))
		.await?;

	println!("Connection closed");
	Ok(())
}
