use crate::prelude::*;
use bevy::prelude::*;
use bytes::Bytes;
use futures::Stream;
use futures::future::BoxFuture;
use send_wrapper::SendWrapper;
use std::pin::Pin;

/// A cross-platform WebSocket that implements Stream of inbound [`Message`].
pub struct Socket {
	pub(crate) incoming: SendWrapper<Pin<Box<DynMessageStream>>>,
	pub(crate) writer: SendWrapper<Box<DynSocketWriter>>,
}

impl std::fmt::Debug for Socket {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Socket").finish_non_exhaustive()
	}
}

impl Socket {
	#[cfg(any(target_arch = "wasm32", feature = "tungstenite"))]
	#[allow(unused_variables)]
	pub async fn connect(
		url: impl AsRef<str>,
	) -> bevy::prelude::Result<Socket> {
		#[cfg(target_arch = "wasm32")]
		{
			super::impl_web_sys::connect_wasm(url).await
		}
		#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
		{
			super::impl_tungstenite::connect_tungstenite(url).await
		}
	}

	/// Create a new socket from a message stream and writer.
	///
	/// This is intended for platform-specific constructors.
	#[cfg(target_arch = "wasm32")]
	pub(crate) fn new(
		incoming: impl Stream<Item = Result<Message>> + 'static,
		writer: Box<DynSocketWriter>,
	) -> Self {
		Self {
			incoming: SendWrapper::new(Box::pin(incoming)),
			writer: SendWrapper::new(writer),
		}
	}

	/// Create a new socket from a message stream and writer.
	///
	/// This is intended for platform-specific constructors.
	#[cfg(not(target_arch = "wasm32"))]
	pub(crate) fn new(
		incoming: impl Stream<Item = Result<Message>> + Send + 'static,
		writer: Box<DynSocketWriter>,
	) -> Self {
		Self {
			incoming: SendWrapper::new(Box::pin(incoming)),
			writer: SendWrapper::new(writer),
		}
	}

	/// Send a message to the peer.
	pub async fn send(&mut self, msg: Message) -> Result<()> {
		self.writer.send_boxed(msg).await
	}

	/// Gracefully close the connection with an optional close frame.
	/// For convenience this does not take the socket, but it should
	/// not be used afterward.
	pub async fn close(&mut self, close: Option<CloseFrame>) -> Result<()> {
		self.writer.close_boxed(close).await
	}

	/// Split this socket into read and write halves.
	///
	/// - The read half implements `Stream<Item = Result<Message>>`.
	/// - The write half provides `send` and `close` methods.
	pub fn split(self) -> (SocketRead, SocketWrite) {
		let read = SocketRead {
			incoming: self.incoming,
		};
		let write = SocketWrite {
			writer: self.writer,
		};
		(read, write)
	}
}

impl Stream for Socket {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.incoming).poll_next(cx)
	}
}


#[cfg(target_arch = "wasm32")]
type DynMessageStream = dyn Stream<Item = Result<Message>>;
/// crates like Axum/Tokio require Stream to be Send on native
#[cfg(not(target_arch = "wasm32"))]
type DynMessageStream = dyn Stream<Item = Result<Message>> + Send;

/// Platform-agnostic writer trait for WebSocket sinks.
///
/// Platform-specific implementations live in their respective modules and are
/// boxed into `Socket`.
pub trait SocketWriter: 'static {
	/// Send a message to the socket peer.
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>>;
	/// Close the socket with an optional close frame.
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>>;
}

#[cfg(target_arch = "wasm32")]
type DynSocketWriter = dyn SocketWriter;
#[cfg(not(target_arch = "wasm32"))]
type DynSocketWriter = dyn SocketWriter + Send;

/// Read half returned by `Socket::split()`.
pub struct SocketRead {
	pub(crate) incoming: SendWrapper<Pin<Box<DynMessageStream>>>,
}

impl Stream for SocketRead {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.incoming).poll_next(cx)
	}
}

/// Write half returned by `Socket::split()`.
pub struct SocketWrite {
	pub(crate) writer: SendWrapper<Box<DynSocketWriter>>,
}

impl SocketWrite {
	/// Send a message to the peer.
	pub async fn send(&mut self, msg: Message) -> Result<()> {
		self.writer.send_boxed(msg).await
	}

	/// Gracefully close the connection with an optional close frame.
	pub async fn close(mut self, close: Option<CloseFrame>) -> Result<()> {
		self.writer.close_boxed(close).await
	}
}


/// A WebSocket message.
///
/// Mirrors common WS message types across platforms (e.g. web-sys and tungstenite)
/// without leaking platform details into your code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
	/// A UTF-8 text message.
	Text(String),
	/// A binary message.
	Binary(Bytes),
	/// A ping frame with an optional payload.
	Ping(Bytes),
	/// A pong frame with an optional payload.
	Pong(Bytes),
	/// A close frame with an optional code and reason.
	Close(Option<CloseFrame>),
}

impl Message {
	/// Create a text message.
	pub fn text(text: impl Into<String>) -> Self { Message::Text(text.into()) }

	/// Create a binary message.
	pub fn binary(bytes: impl Into<Bytes>) -> Self {
		Message::Binary(bytes.into())
	}

	/// Create a ping message.
	pub fn ping(bytes: impl Into<Bytes>) -> Self { Message::Ping(bytes.into()) }

	/// Create a pong message.
	pub fn pong(bytes: impl Into<Bytes>) -> Self { Message::Pong(bytes.into()) }

	/// Create a close message.
	pub fn close(code: u16, reason: impl Into<String>) -> Self {
		Message::Close(Some(CloseFrame {
			code,
			reason: reason.into(),
		}))
	}
}

impl From<&str> for Message {
	fn from(value: &str) -> Self { Message::Text(value.to_owned()) }
}
impl From<String> for Message {
	fn from(value: String) -> Self { Message::Text(value) }
}
impl From<Vec<u8>> for Message {
	fn from(value: Vec<u8>) -> Self { Message::Binary(Bytes::from(value)) }
}
impl From<Bytes> for Message {
	fn from(value: Bytes) -> Self { Message::Binary(value) }
}

/// A close frame describing why a socket was closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseFrame {
	/// Close code as per RFC6455.
	pub code: u16,
	/// Human-readable reason.
	pub reason: String,
}


#[cfg(test)]
mod tests {
	use super::*;
	use beet_core::prelude::*;
	use futures::FutureExt;
	use futures::StreamExt;
	use futures::stream;
	use sweet::prelude::*;

	#[derive(Default, Clone, Copy)]
	struct DummyWriter {
		pub sent: Store<Vec<Message>>,
		pub closed: Store<Option<CloseFrame>>,
	}

	impl SocketWriter for DummyWriter {
		fn send_boxed(
			&mut self,
			msg: Message,
		) -> BoxFuture<'static, Result<()>> {
			self.sent.push(msg);
			async { Ok(()) }.boxed()
		}
		fn close_boxed(
			&mut self,
			close: Option<CloseFrame>,
		) -> BoxFuture<'static, Result<()>> {
			self.closed.set(close);
			async { Ok(()) }.boxed()
		}
	}

	#[sweet::test]
	async fn message_conversions() {
		Message::from("hello")
			.xmap(|m| matches!(m, Message::Text(s) if s == "hello"))
			.xpect_true();

		Message::from(Bytes::from_static(b"\x01\x02"))
			.xmap(
				|m| matches!(m, Message::Binary(b) if b.as_ref() == b"\x01\x02"),
			)
			.xpect_true();

		Message::binary(vec![1, 2, 3])
			.xmap(|m| matches!(m, Message::Binary(b) if b.as_ref() == [1,2,3]))
			.xpect_true();
	}

	#[sweet::test]
	async fn socket_stream_empty() {
		let incoming = stream::empty::<Result<Message>>();
		let mut socket =
			Socket::new(incoming, Box::new(DummyWriter::default()));

		let next = socket.next().await;
		next.is_none().xpect_true();
	}

	#[sweet::test]
	async fn sending_records_messages() {
		let incoming = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let mut socket = Socket::new(incoming, Box::new(writer));

		socket.send(Message::text("hi")).await.unwrap();
		socket.send(Message::binary(vec![9, 8, 7])).await.unwrap();

		writer.sent.len().xpect_eq(2usize);
		matches!(writer.sent.get_index(0).unwrap(), Message::Text(_))
			.xpect_true();
		matches!(writer.sent.get_index(1).unwrap(), Message::Binary(_))
			.xpect_true();
	}

	#[sweet::test]
	async fn closing_records_reason() {
		let incoming = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let mut socket = Socket::new(incoming, Box::new(writer));

		let frame = CloseFrame {
			code: 1000,
			reason: "normal".into(),
		};
		socket.close(Some(frame.clone())).await.unwrap();

		writer.closed.get().unwrap().xpect_eq(frame);
	}

	#[sweet::test]
	async fn split_send_and_read() {
		let incoming = stream::iter(vec![
			Ok(Message::text("a")),
			Ok(Message::binary(vec![1u8, 2, 3])),
		]);

		let writer = DummyWriter::default();
		let socket = Socket::new(incoming, Box::new(writer));

		let (mut read, mut write) = socket.split();

		// send
		write.send(Message::text("hi")).await.unwrap();

		// read stream items
		let m1 = read.next().await.unwrap().unwrap();
		let m2 = read.next().await.unwrap().unwrap();
		matches!(m1, Message::Text(_)).xpect_true();
		matches!(m2, Message::Binary(_)).xpect_true();

		// sent recorded
		writer.sent.len().xpect_eq(1usize);
	}

	#[sweet::test]
	async fn split_close() {
		let incoming = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let socket = Socket::new(incoming, Box::new(writer));

		let (_read, write) = socket.split();

		let frame = CloseFrame {
			code: 1000,
			reason: "bye".into(),
		};
		write.close(Some(frame.clone())).await.unwrap();
		writer.closed.get().unwrap().xpect_eq(frame);
	}

	#[sweet::test]
	// #[ignore="hits public api"]
	async fn echo_endpoint() {
		let url = "wss://echo.websocket.org";
		let mut socket = match Socket::connect(url).await {
			Ok(s) => s,
			Err(e) => panic!("failed to connect to {}: {:?}", url, e),
		};

		let payload = "beet-ws-integration-test";
		socket.send(Message::text(payload)).await.unwrap();

		// only way out is success, error or close
		while let Some(item) = socket.next().await {
			match item {
				Ok(Message::Text(t)) if t == payload => {
					break;
				}
				Ok(_) => continue,
				Err(e) => {
					panic!("error from socket stream: {:?}", e);
				}
			}
		}

		socket.close(None).await.unwrap();
	}
}
