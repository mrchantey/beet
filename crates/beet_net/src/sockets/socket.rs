use beet_core::prelude::*;
use bytes::Bytes;
use futures::Stream;
use futures::future::BoxFuture;
use send_wrapper::SendWrapper;
use std::pin::Pin;

/// A cross-platform WebSocket that implements Stream of inbound [`Message`]
/// and provides methods to send messages and close the connection.
///
#[derive(BundleEffect)]
pub struct Socket {
	// SendWrapper for usage in bevy components
	pub(crate) reader: SendWrapper<Pin<Box<dyn SocketReader>>>,
	// SendWrapper for usage in bevy components
	pub(crate) writer: SendWrapper<Box<dyn SocketWriter>>,
}



impl std::fmt::Debug for Socket {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Socket").finish_non_exhaustive()
	}
}
/// Triggered on a [`Socket`] entity after it has been connected
/// and hooked up.
#[derive(EntityTargetEvent)]
pub struct SocketReady;
impl Socket {
	fn effect(self, entity: &mut EntityWorldMut) {
		let (send, mut recv) = self.split();
		entity
			.observe_any(
				move |ev: On<MessageSend>,
				      mut commands: AsyncCommands|
				      -> Result {
					let mut send = send.clone();
					let message = ev.event().clone();
					commands.run(async move |_| {
						// socket send errors are non-fatal
						send.send(message.take()).await.unwrap_or_else(|err| {
							error!("{:?}", err);
						})
					});
					Ok(())
				},
			)
			.run_async(async move |entity| {
				while let Some(message) = recv.next().await {
					match message {
						Ok(msg) => {
							entity.trigger_target(MessageRecv(msg)).await;
						}
						Err(err) => {
							// socket receive errors break connection but are non-fatal
							error!("{:?}", err);
							break;
						}
					}
				}
			})
			.trigger_target(SocketReady);
	}

	#[allow(unused_variables)]
	pub async fn connect(url: impl AsRef<str>) -> Result<Socket> {
		#[cfg(target_arch = "wasm32")]
		{
			super::impl_web_sys::connect_wasm(url).await
		}
		#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
		{
			super::impl_tungstenite::connect_tungstenite(url).await
		}
		#[cfg(not(any(target_arch = "wasm32", feature = "tungstenite")))]
		{
			panic!(
				"WebSocket implementation not available - enable the tungstenite feature or target wasm32"
			)
		}
	}
	pub fn insert_on_connect(url: impl AsRef<str>) -> OnSpawn {
		let url = url.as_ref().to_owned();
		OnSpawn::new_async_local(async move |entity| -> Result {
			let socket = Socket::connect(url).await?;
			entity.insert_then(socket).await;
			Ok(())
		})
	}

	/// Create a new socket from a message stream and writer.
	#[allow(dead_code)]
	pub(crate) fn new(
		reader: impl SocketReader,
		writer: impl SocketWriter,
	) -> Self {
		Self {
			reader: SendWrapper::new(Box::pin(reader)),
			writer: SendWrapper::new(Box::new(writer)),
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
	pub fn split(self) -> (SocketWrite, SocketRead) {
		let read = SocketRead {
			reader: self.reader,
		};
		let write = SocketWrite {
			writer: self.writer,
		};
		(write, read)
	}
}

impl Stream for Socket {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.reader).poll_next(cx)
	}
}

pub trait SocketReader:
	'static + MaybeSend + Stream<Item = Result<Message>>
{
}
impl<T> SocketReader for T where
	T: 'static + MaybeSend + Stream<Item = Result<Message>>
{
}


/// Read half returned by `Socket::split()`.
pub struct SocketRead {
	pub(crate) reader: SendWrapper<Pin<Box<dyn SocketReader>>>,
}

impl Stream for SocketRead {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.reader).poll_next(cx)
	}
}

/// Platform-agnostic writer trait for WebSocket sinks.
///
/// Platform-specific implementations live in their respective modules and are
/// boxed into `Socket`.
pub trait SocketWriter: 'static + MaybeSend {
	fn clone_boxed(&self) -> Box<dyn SocketWriter>;

	/// Send a message to the socket peer.
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>>;
	/// Close the socket with an optional close frame.
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>>;
}

/// Write half returned by `Socket::split()`.

pub struct SocketWrite {
	pub(crate) writer: SendWrapper<Box<dyn SocketWriter>>,
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
impl Clone for SocketWrite {
	fn clone(&self) -> Self {
		Self {
			writer: SendWrapper::new(self.writer.clone_boxed()),
		}
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

/// A message to be sent by this [`Socket`] writer.
#[derive(Debug, Clone, Deref, PartialEq, Eq, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct MessageSend(pub Message);
impl MessageSend {
	pub fn take(self) -> Message { self.0 }
	pub fn inner(&self) -> &Message { &self.0 }
}

/// A message received by this [`Socket`] reader.
#[derive(Debug, Clone, Deref, PartialEq, Eq, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct MessageRecv(pub Message);
impl MessageRecv {
	pub fn take(self) -> Message { self.0 }
	pub fn inner(&self) -> &Message { &self.0 }
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
#[cfg(any(feature = "tungstenite", target_arch = "wasm32"))]
mod tests {
	use super::*;
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
		fn clone_boxed(&self) -> Box<dyn SocketWriter> {
			Box::new(self.clone())
		}
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
		let reader = stream::empty::<Result<Message>>();
		let mut socket = Socket::new(reader, DummyWriter::default());

		let next = socket.next().await;
		next.is_none().xpect_true();
	}

	#[sweet::test]
	async fn sending_records_messages() {
		let reader = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let mut socket = Socket::new(reader, writer);

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
		let reader = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let mut socket = Socket::new(reader, writer);

		let frame = CloseFrame {
			code: 1000,
			reason: "normal".into(),
		};
		socket.close(Some(frame.clone())).await.unwrap();

		writer.closed.get().unwrap().xpect_eq(frame);
	}

	#[sweet::test]
	async fn split_send_and_read() {
		let reader = stream::iter(vec![
			Ok(Message::text("a")),
			Ok(Message::binary(vec![1u8, 2, 3])),
		]);

		let writer = DummyWriter::default();
		let socket = Socket::new(reader, writer);

		let (mut write, mut read) = socket.split();

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
		let reader = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let socket = Socket::new(reader, writer);

		let (send, _recv) = socket.split();

		let frame = CloseFrame {
			code: 1000,
			reason: "bye".into(),
		};
		send.close(Some(frame.clone())).await.unwrap();
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
