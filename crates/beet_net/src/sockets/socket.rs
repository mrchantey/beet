use crate::prelude::Url;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bevy::platform::sync::Mutex;
use bevy::platform::sync::OnceLock;
use bytes::Bytes;
use core::pin::Pin;
use futures_core::Stream;

cfg_if! {
	if #[cfg(feature = "std")] {
		// On std the real `SendWrapper` enforces thread-affinity at runtime, so the
		// thread-bound reader/writer are only ever touched on their creating thread.
		use send_wrapper::SendWrapper as SocketCell;
	} else {
		// no_std single-threaded stand-in for `send_wrapper::SendWrapper` (which
		// uses `std::thread`): beet runs bevy single-threaded on no_std (no
		// `bevy_multithreaded`), so nothing is ever sent across threads and
		// asserting `Send`/`Sync` for the boxed reader/writer — required for
		// `Socket` to be a valid Component — is sound.
		pub(crate) struct SocketCell<T>(T);
		unsafe impl<T> Send for SocketCell<T> {}
		unsafe impl<T> Sync for SocketCell<T> {}
		impl<T> SocketCell<T> {
			fn new(value: T) -> Self { Self(value) }
		}
		impl<T> core::ops::Deref for SocketCell<T> {
			type Target = T;
			fn deref(&self) -> &T { &self.0 }
		}
		impl<T> core::ops::DerefMut for SocketCell<T> {
			fn deref_mut(&mut self) -> &mut T { &mut self.0 }
		}
	}
}

/// A cross-platform WebSocket that implements Stream of inbound [`Message`]
/// and provides methods to send messages and close the connection.
///
#[derive(BundleEffect)]
pub struct Socket {
	// `SocketCell` (a `SendWrapper` on std, an unsafe no_std stand-in otherwise)
	// so `Socket` is `Send + Sync` for use as a bevy Component despite the
	// thread-bound reader/writer.
	pub(crate) reader: SocketCell<Pin<Box<dyn SocketReader>>>,
	pub(crate) writer: SocketCell<Box<dyn SocketWriter>>,
}

impl core::fmt::Debug for Socket {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		f.debug_struct("Socket").finish_non_exhaustive()
	}
}
/// Triggered on a [`Socket`] entity after it has been connected
/// and hooked up.
#[derive(EntityTargetEvent)]
pub struct SocketReady;

/// Triggered on a [`Socket`] entity when its connection ends, gracefully or
/// not: the reader stream finished (peer close) or yielded an error (transport
/// drop). Fired after the socket's tasks are torn down, so a reconnector (eg
/// [`PersistentSocket`](super::PersistentSocket)) can insert a fresh [`Socket`]
/// on the same entity.
#[derive(EntityTargetEvent)]
pub struct SocketClosed;

/// The entity-lifetime writer feed: the `MessageSend` observer pushes into the
/// current connection's writer channel through this swappable sender, so a
/// replacement [`Socket`] (eg a reconnect) swaps the feed rather than re-wiring
/// the observer. See [`Socket::effect`].
#[derive(Component)]
pub(crate) struct WriterFeed(
	Arc<Mutex<super::writer_channel::Sender<Message>>>,
);

/// Signals this entity's connection end into a channel, alongside the
/// [`SocketClosed`] trigger: insert it (eg a reconnector parking on the
/// receiver) and the reader task sends `()` when its connection ends. A direct
/// channel rather than an observer, so a teardown-time signal is never lost to
/// trigger-dispatch ordering.
#[derive(Clone, Component)]
pub struct SocketClosedNotify(pub super::writer_channel::Sender<()>);

/// Global event: an HTTP backend upgraded a connection to a WebSocket and
/// spawned [`Self::socket`] as an orphan [`Socket`] entity.
///
/// The same-port upgrade seam (`mini_http_server`/`hyper_server`) fires this so
/// the socket layer (eg `client_io`) can adopt the connection, eg re-parent it
/// onto a channel, without the backend knowing about that layer.
#[derive(Debug, Clone, Event)]
pub struct OnWebSocketUpgrade {
	/// The freshly-spawned, fully-wired [`Socket`] entity.
	pub socket: Entity,
}
impl Socket {
	fn effect(self, entity: &mut EntityWorldMut) {
		let (mut send, mut recv) = self.split();
		// Feed the writer task over a channel. Both halves are `SocketCell`s
		// bound to the thread they were created on,
		// so they must only be touched from a `_local` task. The observer runs on
		// an arbitrary pool thread, so it only pushes `Send` messages into the
		// channel rather than touching the writer directly (the channel is
		// `writer_channel`, the agnostic no_std + alloc mpsc).
		let (message_send, message_recv) =
			super::writer_channel::unbounded::<Message>();
		// The `MessageSend` observer lives as long as the entity and pushes into
		// the CURRENT connection's channel through the swappable [`WriterFeed`]:
		// a replacement socket (eg a reconnect) swaps the sender rather than
		// re-wiring the observer (despawning a triggered observer mid-flush is
		// unsound in bevy), and the swapped-out sender's drop ends the previous
		// writer task.
		match entity.get::<WriterFeed>() {
			Some(feed) => *feed.0.lock().unwrap() = message_send,
			None => {
				let feed = Arc::new(Mutex::new(message_send));
				let read_feed = feed.clone();
				let target = entity.id();
				entity.world_scope(move |world| {
					world.spawn(
						Observer::new(move |ev: On<MessageSend>| -> Result {
							read_feed
								.lock()
								.unwrap()
								.send(ev.event().clone().take());
							Ok(())
						})
						.with_entity(target),
					);
				});
				entity.insert(WriterFeed(feed));
			}
		}
		entity
			// writer task: owns `send` and drains the channel on its creation thread.
			.run_async_local(async move |_| {
				while let Some(message) = message_recv.recv().await {
					// socket send errors are non-fatal
					send.send(message).await.unwrap_or_else(|err| {
						error!("{:?}", err);
					})
				}
			})
			// `_local`: the reader is a `SendWrapper` (under `bevy_multithreaded`)
			// bound to the thread it was created on, so it must be polled there.
			.run_async_local(async move |entity| {
				while let Some(message) = recv.next().await {
					match message {
						Ok(msg) => {
							entity.trigger_target(MessageRecv(msg)).await.ok();
						}
						Err(err) => {
							// socket receive errors break connection but are non-fatal
							error!("{:?}", err);
							break;
						}
					}
				}
				// the connection is over: give listeners a synthetic close (an
				// exchange drains its in-flight requests on it), announce the
				// closure, and signal any [`SocketClosedNotify`] channel (eg
				// `PersistentSocket`'s redial loop).
				entity
					.trigger_target(MessageRecv(Message::Close(None)))
					.await
					.ok();
				entity.trigger_target(SocketClosed).await.ok();
				entity
					.get(|notify: &SocketClosedNotify| notify.clone())
					.await
					.ok()
					.map(|notify| notify.0.send(()));
			})
			.trigger_target(SocketReady);
	}

	/// Connects to a WebSocket server at the given [`Url`], eg
	/// `ws://127.0.0.1:8338` (strings convert via `Url::parse`).
	///
	/// Returns a connected [`Socket`] that can be used to send and receive messages.
	#[allow(unused_variables)]
	pub async fn connect(url: impl Into<Url>) -> Result<Socket> {
		let url = url.into();
		cfg_if! {
			if #[cfg(target_arch = "wasm32")] {
				super::impl_web_sys::connect_wasm(&url).await
			} else if #[cfg(feature = "tungstenite")] {
				super::impl_tungstenite::connect_tungstenite(&url).await
			} else {
				// no backend compiled in: defer to a transport installed at
				// runtime via `set_socket_client` (eg the esp WiFi adapter),
				// mirroring `send_http`'s bare-metal fallthrough.
				match SOCKET_CLIENT.get() {
					Some(connect) => connect(url).await,
					None => bevybail!(
						"No WebSocket transport configured. Enable the tungstenite \
						 feature, target wasm32, or install one via set_socket_client(...)."
					),
				}
			}
		}
	}
	/// Returns an [`OnSpawn`] callback that connects to the URL and inserts the socket.
	pub fn insert_on_connect(url: impl Into<Url>) -> OnSpawn {
		let url = url.into();
		OnSpawn::new_async_local(async move |entity| -> Result {
			let socket = Socket::connect(url).await?;
			entity.insert(socket).await?;
			Ok(())
		})
	}

	/// Create a [`Socket`] from a message stream reader and a writer.
	///
	/// The seam a downstream transport uses to build a `Socket` from its own
	/// channel ends after installing [`set_socket_client`] (the esp WiFi backend
	/// does exactly this); the built-in tungstenite/web-sys backends use it too.
	pub fn new(
		reader: impl SocketReader,
		writer: impl SocketWriter,
	) -> Self {
		Self {
			reader: SocketCell::new(Box::pin(reader)),
			writer: SocketCell::new(Box::new(writer)),
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

/// A runtime-installed WebSocket connect transport, mirroring `HttpSendFn`.
///
/// The no_std-friendly client hook: when no backend is compiled in (`tungstenite`
/// or wasm/web-sys), [`Socket::connect`] falls through to a function installed
/// here, letting a bare-metal adapter (an esp WiFi crate, …) plug in its own
/// transport without living in `beet_net`. Owns the [`Url`] so the returned
/// future carries it; `url.host()`/`url.port_or_default()` split the authority.
pub type SocketConnectFn =
	fn(url: Url) -> MaybeSendBoxedFuture<'static, Result<Socket>>;

static SOCKET_CLIENT: OnceLock<SocketConnectFn> = OnceLock::new();

/// Install the WebSocket transport [`Socket::connect`] uses when no client
/// backend is compiled in, mirroring `set_http_client`. Errors if one is already
/// installed.
pub fn set_socket_client(client: SocketConnectFn) -> Result<()> {
	SOCKET_CLIENT
		.set(client)
		.map_err(|_| bevyhow!("Socket client already installed"))
}

impl Stream for Socket {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.reader).poll_next(cx)
	}
}

/// Trait for WebSocket message streams.
///
/// Implemented automatically for any type that is `'static + MaybeSend` and
/// implements `Stream<Item = Result<Message>>`.
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
	pub(crate) reader: SocketCell<Pin<Box<dyn SocketReader>>>,
}

impl Stream for SocketRead {
	type Item = Result<Message>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut core::task::Context<'_>,
	) -> core::task::Poll<Option<Self::Item>> {
		Pin::new(&mut self.reader).poll_next(cx)
	}
}

/// Platform-agnostic writer trait for WebSocket sinks.
///
/// Platform-specific implementations live in their respective modules and are
/// boxed into `Socket`.
pub trait SocketWriter: 'static + MaybeSend {
	/// Send a message to the socket peer.
	fn send_boxed(
		&mut self,
		msg: Message,
	) -> SendBoxedFuture<Result<()>>;
	/// Close the socket with an optional close frame.
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> SendBoxedFuture<Result<()>>;
}

/// Write half returned by `Socket::split()`.

pub struct SocketWrite {
	pub(crate) writer: SocketCell<Box<dyn SocketWriter>>,
}

impl SocketWrite {
	/// Send a message to the peer.
	pub async fn send(&mut self, msg: Message) -> Result<()> {
		self.writer.send_boxed(msg).await
	}

	/// Gracefully close the connection with an optional close frame.
	pub async fn close(&mut self, close: Option<CloseFrame>) -> Result<()> {
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

/// A message to be sent by this [`Socket`] writer.
#[derive(Debug, Clone, Deref, PartialEq, Eq, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct MessageSend(pub Message);
impl MessageSend {
	/// Consumes self and returns the inner [`Message`].
	pub fn take(self) -> Message { self.0 }
	/// Returns a reference to the inner [`Message`].
	pub fn inner(&self) -> &Message { &self.0 }
}

/// A message received by this [`Socket`] reader.
#[derive(Debug, Clone, Deref, PartialEq, Eq, EntityTargetEvent)]
#[event(auto_propagate)]
pub struct MessageRecv(pub Message);
impl MessageRecv {
	/// Consumes self and returns the inner [`Message`].
	pub fn take(self) -> Message { self.0 }
	/// Returns a reference to the inner [`Message`].
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
	use futures::StreamExt;
	use futures::stream;

	#[derive(Default, Clone, Copy)]
	struct DummyWriter {
		pub sent: Store<Vec<Message>>,
		pub closed: Store<Option<CloseFrame>>,
	}

	impl SocketWriter for DummyWriter {
		fn send_boxed(
			&mut self,
			msg: Message,
		) -> SendBoxedFuture<Result<()>> {
			self.sent.push(msg);
			Box::pin(async { Ok(()) })
		}
		fn close_boxed(
			&mut self,
			close: Option<CloseFrame>,
		) -> SendBoxedFuture<Result<()>> {
			self.closed.set(close);
			Box::pin(async { Ok(()) })
		}
	}

	#[beet_core::test]
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

	#[beet_core::test]
	async fn socket_stream_empty() {
		let reader = stream::empty::<Result<Message>>();
		let mut socket = Socket::new(reader, DummyWriter::default());

		let next = socket.next().await;
		next.is_none().xpect_true();
	}

	#[beet_core::test]
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

	#[beet_core::test]
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

	#[beet_core::test]
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

	#[beet_core::test]
	async fn split_close() {
		let reader = stream::empty::<Result<Message>>();
		let writer = DummyWriter::default();
		let socket = Socket::new(reader, writer);

		let (mut send, _recv) = socket.split();

		let frame = CloseFrame {
			code: 1000,
			reason: "bye".into(),
		};
		send.close(Some(frame.clone())).await.unwrap();
		writer.closed.get().unwrap().xpect_eq(frame);
	}

	#[beet_core::test]
	#[cfg(all(feature = "tungstenite", not(target_arch = "wasm32")))]
	async fn echo_endpoint() {
		use crate::sockets::echo_socket_server::EchoSocketServer;

		let server = EchoSocketServer::new().await;
		let mut socket =
			Socket::connect(&server.url().to_string()).await.unwrap();

		let payload = "beet-ws-integration-test";
		socket.send(Message::text(payload)).await.unwrap();

		while let Some(item) = socket.next().await {
			match item {
				Ok(Message::Text(text)) if text == payload => {
					break;
				}
				Ok(_) => continue,
				Err(err) => {
					panic!("error from socket stream: {:?}", err);
				}
			}
		}

		socket.close(None).await.unwrap();
	}
}
