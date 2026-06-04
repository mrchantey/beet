//! Local echo WebSocket server for testing.

use crate::prelude::*;
use crate::sockets::common_handlers::echo_close;
use crate::sockets::common_handlers::echo_message;
use crate::sockets::*;
use beet_core::prelude::*;

/// A local echo WebSocket server for integration tests.
///
/// Echoes back any Text or Binary message, and acknowledges Close frames.
/// Uses the existing [`echo_message`] and [`echo_close`] handlers.
pub struct EchoSocketServer {
	/// The WebSocket URL of the running server, ie `ws://127.0.0.1:38501`.
	url: Url,
}

impl EchoSocketServer {
	/// The base [`Url`] of the running server.
	pub fn url(&self) -> &Url { &self.url }

	/// Starts a new echo WebSocket server on a background thread.
	///
	/// Returns immediately after the server is ready to accept connections.
	pub async fn new() -> Self {
		let server = SocketServer::new_test();
		let url = Url::parse(&server.0.local_url());

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, SocketServerPlugin::default()));
			app.world_mut()
				.spawn(server)
				.observe_any(echo_message)
				.observe_any(echo_close);
			app.run();
		});
		time_ext::sleep_millis(200).await;

		Self { url }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sockets::Message;
	use futures::StreamExt;

	#[beet_core::test]
	async fn echo_text() {
		let server = EchoSocketServer::new().await;
		let mut socket =
			Socket::connect(&server.url().to_string()).await.unwrap();

		socket.send(Message::text("hello")).await.unwrap();

		while let Some(item) = socket.next().await {
			match item.unwrap() {
				Message::Text(text) => {
					text.xpect_eq("hello");
					break;
				}
				_ => continue,
			}
		}

		socket.close(None).await.ok();
	}

	#[beet_core::test]
	async fn echo_binary() {
		let server = EchoSocketServer::new().await;
		let mut socket =
			Socket::connect(&server.url().to_string()).await.unwrap();

		let payload: Vec<u8> = vec![0xDE, 0xAD, 0xBE, 0xEF];
		socket.send(Message::binary(payload.clone())).await.unwrap();

		while let Some(item) = socket.next().await {
			match item.unwrap() {
				Message::Binary(data) => {
					data.as_ref().xpect_eq(payload.as_slice());
					break;
				}
				_ => continue,
			}
		}

		socket.close(None).await.ok();
	}

	#[beet_core::test]
	async fn echo_close_frame() {
		let server = EchoSocketServer::new().await;
		let mut socket =
			Socket::connect(&server.url().to_string()).await.unwrap();

		let frame = CloseFrame {
			code: 1000,
			reason: "normal closure".into(),
		};
		socket
			.send(Message::Close(Some(frame.clone())))
			.await
			.unwrap();

		while let Some(item) = socket.next().await {
			match item.unwrap() {
				Message::Close(payload) => {
					payload.unwrap().xpect_eq(frame);
					break;
				}
				_ => continue,
			}
		}
	}

	#[beet_core::test]
	async fn multiple_messages() {
		let server = EchoSocketServer::new().await;
		let mut socket =
			Socket::connect(&server.url().to_string()).await.unwrap();

		let messages: Vec<Message> = vec![
			Message::text("first"),
			Message::text("second"),
			Message::binary(vec![1, 2, 3]),
			Message::text("third"),
		];

		for msg in &messages {
			socket.send(msg.clone()).await.unwrap();
		}

		let mut received: Vec<Message> = Vec::new();
		while let Some(item) = socket.next().await {
			match item.unwrap() {
				msg @ (Message::Text(_) | Message::Binary(_)) => {
					received.push(msg);
					if received.len() == messages.len() {
						break;
					}
				}
				_ => continue,
			}
		}

		received.len().xpect_eq(messages.len());
		for (idx, sent) in messages.iter().enumerate() {
			received[idx].xpect_eq(sent.clone());
		}

		socket.close(None).await.ok();
	}
}
