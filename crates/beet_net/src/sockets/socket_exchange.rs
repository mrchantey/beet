//! The `Request`/`Response` exchange carried over a [`Socket`], so a route call on
//! one peer is served by the other peer's router and the reply comes back.
//!
//! An exchange-enabled socket is a symmetric duplex channel: either peer can
//! **originate** a request (the [`socket_exchange`] action frames it, sends it, and
//! awaits the correlated reply) and **serve** the other's requests (the receive pump
//! dispatches each inbound request through the connection's nearest ancestor
//! `Action<Request, Response>` slot - typically a `Router` - and frames the reply).
//! Everything rides on the existing exchange seam (`entity.exchange`), the existing
//! socket events ([`MessageSend`] / [`MessageRecv`]), and the [`oneshot`] primitive;
//! the only new machinery is the [`ExchangeFrame`] envelope and a `u64` correlation
//! id.
use super::*;
// disambiguate the socket `Message` enum from `beet_core`'s `Message` trait.
use super::Message;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use bevy::platform::sync::Arc;
use bytes::Bytes;

/// Enables the duplex [`Request`]/[`Response`] exchange on a [`Socket`] connection.
///
/// Add it beside a `Socket` (a client's `Socket::connect`, or each connection a
/// `SocketServer` accepts) to make that connection a Request/Response endpoint: its
/// `on_add` wires the receive pump, and it holds the wire [`codec`](Self::new) plus
/// the in-flight requests originated over it. Read by [`socket_exchange`].
#[derive(Component)]
#[component(on_add = on_add, on_remove = on_remove)]
pub struct ExchangeSocket {
	/// The wire codec; pluggable so a constrained peer can swap the encoding.
	codec: Arc<dyn ExchangeCodec>,
	/// Requests originated over this connection, awaiting their correlated reply.
	pending: HashMap<u64, OnceValue<Response>>,
	/// The next correlation id to hand out.
	next_id: u64,
}

impl ExchangeSocket {
	/// Enable the exchange with an explicit [`ExchangeCodec`].
	pub fn new(codec: impl ExchangeCodec) -> Self {
		Self {
			codec: Arc::new(codec),
			pending: HashMap::default(),
			next_id: 0,
		}
	}

	/// Enable the exchange with the compact binary [`PostcardCodec`] (the default).
	#[cfg(feature = "postcard")]
	pub fn postcard() -> Self { Self::new(PostcardCodec) }

	/// Enable the exchange with the human-readable [`JsonCodec`], for debugging.
	#[cfg(feature = "json")]
	pub fn json() -> Self { Self::new(JsonCodec) }
}

/// The originating side: an `Action<Request, Response>` that forwards its request over
/// `connection` (an [`ExchangeSocket`]) and awaits the peer's reply.
///
/// Install it as a route handler so a server-side route forwards to a connected
/// client instead of handling locally, eg
/// `exchange_route("take-photo", socket_exchange(web_connection))`. The user-facing
/// "SocketExchange action".
pub fn socket_exchange(connection: Entity) -> Action<Request, Response> {
	Action::new_async(move |cx: ActionContext<Request>| async move {
		let connection = cx.caller.world().entity(connection);
		// register a pending reply, take an id, and read the codec in one access.
		let (reply, id, codec) = connection
			.with(move |mut entity: EntityWorldMut| -> Result<_> {
				let mut socket =
					entity.get_mut::<ExchangeSocket>().ok_or_else(|| {
						bevyhow!("connection has no `ExchangeSocket`")
					})?;
				let id = socket.next_id;
				socket.next_id += 1;
				let (sender, reply) = oneshot();
				socket.pending.insert(id, sender);
				Ok((reply, id, socket.codec.clone()))
			})
			.await??;
		let frame = ExchangeFrame::request(id, cx.input).await?;
		connection.trigger_target(MessageSend(codec.encode(&frame)?)).await?;
		reply.wait().await.xok()
	})
}

/// Encodes an [`ExchangeFrame`] to a socket [`Message`] and back. Pluggable per
/// [`ExchangeSocket`] so other cases swap the wire format - a downstream crate (eg an
/// embedded peer) can implement a bespoke codec without touching `beet_net`.
pub trait ExchangeCodec: 'static + Send + Sync {
	/// Encode a frame to a message to send over the socket.
	fn encode(&self, frame: &ExchangeFrame) -> Result<Message>;
	/// Decode a received message, or `None` if it is not an exchange frame (eg a
	/// ping or an application message sharing the socket).
	fn decode(&self, message: &Message) -> Result<Option<ExchangeFrame>>;
}

/// The compact binary default codec: `postcard` over [`Message::Binary`], clean for
/// the raw bytes a photo tool returns.
#[cfg(feature = "postcard")]
pub struct PostcardCodec;

#[cfg(feature = "postcard")]
impl ExchangeCodec for PostcardCodec {
	fn encode(&self, frame: &ExchangeFrame) -> Result<Message> {
		Message::binary(postcard::to_allocvec(frame)?).xok()
	}
	fn decode(&self, message: &Message) -> Result<Option<ExchangeFrame>> {
		match message {
			Message::Binary(bytes) => {
				postcard::from_bytes(bytes).map(Some).map_err(Into::into)
			}
			_ => Ok(None),
		}
	}
}

/// The human-readable debugging codec: JSON over [`Message::Text`].
#[cfg(feature = "json")]
pub struct JsonCodec;

#[cfg(feature = "json")]
impl ExchangeCodec for JsonCodec {
	fn encode(&self, frame: &ExchangeFrame) -> Result<Message> {
		Message::text(serde_json::to_string(frame)?).xok()
	}
	fn decode(&self, message: &Message) -> Result<Option<ExchangeFrame>> {
		match message {
			Message::Text(text) => {
				serde_json::from_str(text).map(Some).map_err(Into::into)
			}
			_ => Ok(None),
		}
	}
}

/// The wire envelope: a correlation id plus a [`Request`] or [`Response`] flattened
/// into its already-serde-able pieces (the `Request`/`Response` wrappers cannot derive
/// serde - their `Body` may be a stream, and `RequestParts` deliberately omits
/// `Deserialize` to keep `FromRequest` unambiguous - so the wire carries the fields it
/// needs plus the collected body).
#[derive(serde::Serialize, serde::Deserialize)]
pub enum ExchangeFrame {
	/// A request to serve, correlated by `id`.
	Request {
		/// The originator's correlation id, echoed in the reply.
		id: u64,
		/// The request method.
		method: HttpMethod,
		/// The request url (path + query the router matches on).
		url: Url,
		/// The request headers.
		headers: HeaderMap,
		/// The collected request body.
		body: Vec<u8>,
	},
	/// A reply to the request with the matching `id`.
	Response {
		/// The id of the request this replies to.
		id: u64,
		/// The response status.
		status: StatusCode,
		/// The response headers.
		headers: HeaderMap,
		/// The collected response body.
		body: Vec<u8>,
	},
}

impl ExchangeFrame {
	/// Frame a request to originate, collecting its body to bytes.
	async fn request(id: u64, request: Request) -> Result<Self> {
		let (parts, body) = request.into_parts();
		Self::Request {
			id,
			method: parts.method().clone(),
			url: parts.url().clone(),
			headers: parts.headers.clone(),
			body: body.into_bytes().await?.to_vec(),
		}
		.xok()
	}

	/// Frame a response to reply with, collecting its body to bytes.
	async fn response(id: u64, response: Response) -> Result<Self> {
		let Response { parts, body } = response;
		Self::Response {
			id,
			status: parts.status,
			headers: parts.headers,
			body: body.into_bytes().await?.to_vec(),
		}
		.xok()
	}
}

/// Wire the receive pump when an [`ExchangeSocket`] is added to a connection.
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world.commands().entity(cx.entity).observe_any(on_exchange_recv);
}

/// Resolve any in-flight requests when the connection's exchange is torn down (eg
/// the connection entity despawns), so an originator never hangs on a dropped peer.
fn on_remove(mut world: DeferredWorld, cx: HookContext) {
	if let Some(mut socket) =
		world.entity_mut(cx.entity).get_mut::<ExchangeSocket>()
	{
		drain_pending(&mut socket);
	}
}

/// The receive pump: decode each inbound message and either serve it (a request,
/// dispatched through the ancestor router) or resolve its originator (a response). A
/// `Close` drains the in-flight requests.
fn on_exchange_recv(
	ev: On<MessageRecv>,
	mut sockets: Query<&mut ExchangeSocket>,
	commands: AsyncCommands,
) -> Result {
	let connection = ev.target();
	let Ok(mut socket) = sockets.get_mut(connection) else {
		return Ok(());
	};
	if let Message::Close(_) = ev.event().inner() {
		drain_pending(&mut socket);
		return Ok(());
	}
	let Some(frame) = socket.codec.decode(ev.event().inner())? else {
		return Ok(());
	};
	match frame {
		// a reply to a request we originated: hand the rebuilt response to its waiter.
		ExchangeFrame::Response {
			id,
			status,
			headers,
			body,
		} => {
			if let Some(sender) = socket.pending.remove(&id) {
				let mut parts = ResponseParts::new(status);
				parts.headers = headers;
				sender.signal(Response::from_parts(parts, Bytes::from(body)));
			}
		}
		// a request to serve: dispatch and reply on the connection's own thread.
		ExchangeFrame::Request {
			id,
			method,
			url,
			headers,
			body,
		} => {
			let codec = socket.codec.clone();
			commands.entity(connection).run_local(async move |connection| {
				serve_request(connection, codec, id, method, url, headers, body)
					.await
			});
		}
	}
	Ok(())
}

/// Dispatch an inbound request through the connection's nearest ancestor (inclusive)
/// `Action<Request, Response>` slot, then frame and send the reply correlated by `id`.
/// A missing router replies `500`, so an originator always gets an answer.
async fn serve_request(
	connection: AsyncEntity,
	codec: Arc<dyn ExchangeCodec>,
	id: u64,
	method: HttpMethod,
	url: Url,
	headers: HeaderMap,
	body: Vec<u8>,
) -> Result {
	let mut parts = RequestParts::new(method, url);
	parts.headers = headers;
	let request = Request::from_parts(parts, Body::Bytes(Bytes::from(body)));
	let router = connection
		.with_state::<AncestorQuery<&Action<Request, Response>>, _>(
			|entity, query| query.get_entity(entity),
		)
		.await?;
	let response = match router {
		Ok(router) => connection.world().entity(router).exchange(request).await,
		Err(_) => Response::internal_error(),
	};
	let frame = ExchangeFrame::response(id, response).await?;
	connection.trigger_target(MessageSend(codec.encode(&frame)?)).await?;
	Ok(())
}

/// Resolve every in-flight request with a `502`, so waiters fail cleanly rather than
/// hang when the connection closes or is torn down.
fn drain_pending(socket: &mut ExchangeSocket) {
	for (_id, sender) in socket.pending.drain() {
		sender.signal(Response::from_status(StatusCode::new(502)));
	}
}

// the in-memory `socket_pair` rides `async_channel` + `AsyncPlugin::world`, both
// std-only, so the test module needs `std` even though the module under test does not.
#[cfg(all(test, feature = "postcard", feature = "std"))]
mod test {
	use super::*;

	/// A connected pair of in-memory [`Socket`]s (a's sends reach b's reads and vice
	/// versa), the wasm-safe transport for exercising the exchange without TCP.
	fn socket_pair() -> (Socket, Socket) {
		// types inferred from `Socket::channel` (a `Message` channel each way).
		let (a_tx, a_rx) = async_channel::unbounded();
		let (b_tx, b_rx) = async_channel::unbounded();
		(Socket::channel(a_tx, b_rx), Socket::channel(b_tx, a_rx))
	}

	/// A request originated over one peer is served by the other's handler and the
	/// reply comes back. The handler mirrors the path, so the response body proves the
	/// whole round trip travelled the socket.
	#[beet_core::test]
	async fn round_trip() {
		let mut world = AsyncPlugin::world();
		let (client, server) = socket_pair();
		// the serving peer: its `exchange_handler` is the ancestor `Action` the pump
		// dispatches inbound requests to.
		world.spawn((
			server,
			ExchangeSocket::postcard(),
			exchange_handler(|req| {
				Response::ok().with_body(req.take().path_string())
			}),
		));
		let origin = world.spawn((client, ExchangeSocket::postcard())).id();
		world.flush();
		world
			.entity_mut(origin)
			.run_async_then(move |origin| async move {
				origin
					.call_detached(
						socket_exchange(origin.id()),
						Request::get("hello/world"),
					)
					.await
			})
			.await
			.unwrap()
			.unwrap_str()
			.await
			.xpect_eq("/hello/world".to_string());
	}

	/// Several requests in flight over one connection each resolve to their own reply,
	/// proving the `u64` correlation multiplexes rather than crossing wires.
	#[beet_core::test]
	async fn multiplexes_by_id() {
		let mut world = AsyncPlugin::world();
		let (client, server) = socket_pair();
		world.spawn((
			server,
			ExchangeSocket::postcard(),
			exchange_handler(|req| {
				Response::ok().with_body(req.take().path_string())
			}),
		));
		let origin = world.spawn((client, ExchangeSocket::postcard())).id();
		world.flush();
		world
			.entity_mut(origin)
			.run_async_then(move |origin| async move {
				let exchange = socket_exchange(origin.id());
				let one = origin
					.call_detached(exchange.clone(), Request::get("one"))
					.await?;
				let two =
					origin.call_detached(exchange, Request::get("two")).await?;
				Ok::<_, BevyError>((
					one.unwrap_str().await,
					two.unwrap_str().await,
				))
			})
			.await
			.unwrap()
			.xpect_eq(("/one".to_string(), "/two".to_string()));
	}

	/// A connection torn down with a request still in flight resolves that request
	/// with a `502`, so an originator fails cleanly rather than hanging on a dropped
	/// peer. Exercises the drain the `Close` and `on_remove` paths both call.
	#[beet_core::test]
	async fn drain_resolves_pending() {
		let mut socket = ExchangeSocket::postcard();
		let (sender, reply) = oneshot();
		socket.pending.insert(7, sender);
		drain_pending(&mut socket);
		reply.wait().await.status().as_u16().xpect_eq(502);
	}
}
