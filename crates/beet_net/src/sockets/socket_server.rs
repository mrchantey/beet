use crate::prelude::sockets::*;
use beet_core::prelude::*;
use futures::Stream;
use std::pin::Pin;

/// A WebSocket server that can accept incoming connections.
///
/// Platform-specific implementations provide the actual binding and accept logic.
/// Each accepted connection returns a [`Socket`] that can be used like any client socket.
pub struct SocketServer {
	pub(crate) acceptor: Box<DynSocketAcceptor>,
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
		#[cfg(feature = "tungstenite")]
		{
			super::impl_tungstenite::bind_tungstenite(_addr).await
		}
		#[cfg(not(feature = "tungstenite"))]
		panic!("WebSocket server requires the 'tungstenite' feature")
	}

	/// Create a new server with the given adaptor.
	/// For one created based on features see [`Self::bind`]
	///
	/// This is intended for platform-specific constructors.
	pub fn new(acceptor: Box<DynSocketAcceptor>) -> Self { Self { acceptor } }


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
pub trait SocketAcceptor: Stream<Item = Result<Socket>> + 'static {
	/// Accept a new incoming WebSocket connection.
	fn accept(
		&mut self,
	) -> Pin<Box<dyn std::future::Future<Output = Result<Socket>> + Send + '_>>;

	/// Get the local address this acceptor is bound to.
	fn local_addr(&self) -> Result<std::net::SocketAddr>;
}

#[cfg(not(target_arch = "wasm32"))]
type DynSocketAcceptor = dyn SocketAcceptor + Send;

#[cfg(target_arch = "wasm32")]
type DynSocketAcceptor = dyn SocketAcceptor;


#[cfg(test)]
#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
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
}
