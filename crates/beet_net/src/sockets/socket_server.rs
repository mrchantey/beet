use crate::prelude::sockets::*;
use beet_core::async_ext::NativeSendBoxedFuture;
use beet_core::prelude::*;
use futures::Stream;
use std::pin::Pin;

/// A WebSocket server that can accept incoming connections.
///
/// Platform-specific implementations provide the actual binding and accept logic.
/// Each accepted connection returns a [`Socket`] that can be used like any client socket.
pub struct SocketServer {
	pub(crate) acceptor: Box<dyn SocketAcceptor>,
}

impl std::fmt::Debug for SocketServer {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("SocketServer").finish_non_exhaustive()
	}
}

impl SocketServer {
	/// Bind a WebSocket server to the given address.
	///
	/// Returns a `SocketServer` that can accept incoming connections.
	///
	/// Use "127.0.0.1:0" to bind to any available port, then call `local_addr()` to get the actual address.
	pub async fn bind(_addr: impl AsRef<str>) -> Result<Self> {
		#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
		{
			super::impl_tungstenite::bind_tungstenite(_addr).await
		}
		#[cfg(not(all(feature = "tungstenite", not(target_arch = "wasm32"))))]
		panic!("WebSocket server requires the 'tungstenite' feature")
	}

	/// Create a new server with the given adaptor.
	/// For one created based on features see [`Self::bind`]
	///
	/// This is intended for platform-specific constructors.
	pub fn new(acceptor: impl SocketAcceptor) -> Self {
		Self {
			acceptor: Box::new(acceptor),
		}
	}


	/// Get the local address this server is bound to.
	///
	/// This is useful when binding to port 0 to discover which port was assigned.
	pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
		self.acceptor.local_addr()
	}

	/// Accept a new WebSocket connection.
	///
	/// This waits for an incoming connection, performs the WebSocket handshake,
	/// and returns a [`Socket`] ready to send and receive messages.
	pub async fn accept(&mut self) -> Result<Socket> {
		self.acceptor.accept().await
	}
}

impl Stream for SocketServer {
	type Item = Result<Socket>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		// SAFETY: acceptor is pinned because SocketServer is pinned and we never move it
		unsafe { Pin::new_unchecked(&mut *self.acceptor).poll_next(cx) }
	}
}

/// Platform-agnostic acceptor trait for WebSocket servers.
///
/// Platform-specific implementations live in their respective modules and are
/// boxed into `SocketServer`.
pub trait SocketAcceptor:
	'static + MaybeSend + Stream<Item = Result<Socket>>
{
	/// Accept a new incoming WebSocket connection.
	fn accept(&mut self) -> NativeSendBoxedFuture<'_, Result<Socket>>;

	/// Get the local address this acceptor is bound to.
	fn local_addr(&self) -> Result<std::net::SocketAddr>;
}

#[cfg(test)]
#[cfg(all(
	feature = "tungstenite",
	feature = "multi_threaded",
	not(target_arch = "wasm32")
))]
mod tests {
	use super::super::Message;
	use super::*;
	use sweet::prelude::*;


	#[sweet::test]
	async fn server_binds_and_accepts() {
		let mut server = SocketServer::bind("127.0.0.1:0").await.unwrap();
		let addr = server.local_addr().unwrap();

		let server_task = async {
			let mut socket = server.next().await.unwrap().unwrap();
			socket.next().await.unwrap().unwrap()
		};

		let client_task = async {
			// give server time to start accepting
			time_ext::sleep_millis(100).await;
			let url = format!("ws://{}", addr);
			let mut client = Socket::connect(&url).await.unwrap();
			client.send(Message::text("hello server")).await.unwrap();
			client.close(None).await.ok();
		};

		let (msg, _) = tokio::join!(server_task, client_task);
		matches!(msg, Message::Text(ref s) if s == "hello server").xpect_true();
	}

	#[sweet::test]
	async fn handles_multiple_concurrent_connections() {
		let mut server = SocketServer::bind("127.0.0.1:0").await.unwrap();
		let addr = server.local_addr().unwrap();

		let server_task = async move {
			// Accept first client and block on it for a bit
			let mut socket1 = server.next().await.unwrap().unwrap();
			let msg1 = socket1.next().await.unwrap().unwrap();

			// Send response and keep connection alive
			socket1.send(msg1).await.unwrap();
			time_ext::sleep_millis(500).await;
			socket1.close(None).await.ok();

			// Accept second client (should succeed even though first is still connected)
			let mut socket2 = server.next().await.unwrap().unwrap();
			let msg2 = socket2.next().await.unwrap().unwrap();
			socket2.send(msg2).await.unwrap();
			socket2.close(None).await.ok();
		};

		let client1_task = async {
			time_ext::sleep_millis(50).await;
			let url = format!("ws://{}", addr);
			let mut client = Socket::connect(&url).await.unwrap();
			client.send(Message::text("client1")).await.unwrap();
			let response = client.next().await.unwrap().unwrap();
			matches!(response, Message::Text(ref s) if s == "client1")
				.xpect_true();
			client.close(None).await.ok();
		};

		let client2_task = async {
			time_ext::sleep_millis(100).await;
			let url = format!("ws://{}", addr);

			// This connection should succeed even though first client is still active
			let mut client = Socket::connect(&url).await.unwrap();
			client.send(Message::text("client2")).await.unwrap();
			let response = client.next().await.unwrap().unwrap();
			matches!(response, Message::Text(ref s) if s == "client2")
				.xpect_true();
			client.close(None).await.ok();
		};

		let _ = tokio::join!(server_task, client1_task, client2_task);
	}
}
