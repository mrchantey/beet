//! In-memory WebSocket server: accepts "connections" delivered over an
//! [`async_channel`] instead of TCP, adopting each as a [`Socket`] exactly as the
//! tungstenite accept path adopts a real connection.
use crate::prelude::*;
use crate::sockets::*;
// explicit so the socket `Message` enum wins over bevy's `Message` trait (both are
// in scope via the prelude globs).
use crate::sockets::Message;
use async_channel::Receiver;
use async_channel::Sender;
use beet_action::prelude::*;
use beet_core::prelude::*;
use futures::FutureExt;
use futures::future::BoxFuture;

/// A self-contained WebSocket server that accepts connections delivered over a
/// channel rather than a socket, adopting each as a child [`Socket`] dispatched
/// through the entity's [`MessageRecv`] / [`MessageSend`] events.
///
/// The socket analogue of [`ChannelHttpServer`]: a per-instance component (no
/// global backend), so multiple can coexist, with the same wasm-first / mock /
/// deterministic-test use. Boots through the fan-out like [`SocketServer`] - a
/// [`StartRunning<Boot>`] whose `--server` selects `"channel"` starts the accept
/// loop, which parks on the host's [`Running<Response>`] keep-alive and tears down
/// on its removal.
///
/// Runtime-only: it holds an [`async_channel`] end, which is not [`Reflect`], so it
/// is not markup-spawnable. Construct it with [`ChannelSocketServer::new`].
#[derive(Component)]
#[component(on_add = on_add)]
#[require(ContinueRun<Boot, Response>)]
pub struct ChannelSocketServer {
	/// Incoming connections; each yields a fresh server-side [`Socket`].
	connections: Receiver<ChannelSocketConn>,
}

/// The user-held end of a [`ChannelSocketServer`]: open a connection to spawn a new
/// [`Socket`] on the server and obtain the matching client-side [`Socket`].
pub struct ChannelSocketClient {
	/// Outbound connection requests.
	connections: Sender<ChannelSocketConn>,
}

/// A pending connection delivered to a [`ChannelSocketServer`]: the server-side
/// channel ends that become an accepted [`Socket`]. Internal to the connection
/// protocol; only [`ChannelSocketClient::connect`] constructs one.
struct ChannelSocketConn {
	/// Server -> client messages.
	tx: Sender<Message>,
	/// Client -> server messages.
	rx: Receiver<Message>,
}

impl ChannelSocketServer {
	/// Creates a paired server and client over a fresh connections channel.
	pub fn new() -> (ChannelSocketServer, ChannelSocketClient) {
		let (conn_tx, conn_rx) = async_channel::unbounded::<ChannelSocketConn>();
		(
			ChannelSocketServer {
				connections: conn_rx,
			},
			ChannelSocketClient {
				connections: conn_tx,
			},
		)
	}
}

impl ChannelSocketClient {
	/// Open a new connection, returning the client-side [`Socket`].
	///
	/// Builds the two message channels (client->server, server->client), hands the
	/// server its ends over the connections channel, and returns a [`Socket`] over
	/// the mirror ends. The server adopts its side when its accept loop is driven.
	pub async fn connect(&self) -> Result<Socket> {
		let (c2s_tx, c2s_rx) = async_channel::unbounded::<Message>();
		let (s2c_tx, s2c_rx) = async_channel::unbounded::<Message>();
		self.connections
			.send(ChannelSocketConn {
				tx: s2c_tx,
				rx: c2s_rx,
			})
			.await
			.map_err(|_| bevyhow!("channel socket server closed"))?;
		Ok(Socket::channel(c2s_tx, s2c_rx))
	}
}

impl Socket {
	/// Build a [`Socket`] over a pair of [`async_channel`] message ends: it reads
	/// inbound [`Message`]s from `rx` and writes outbound ones to `tx`.
	///
	/// The reusable channel-transport primitive behind [`ChannelSocketServer`]; the
	/// client side of a channel connection is just another `Socket::channel` over
	/// the mirror ends.
	pub fn channel(tx: Sender<Message>, rx: Receiver<Message>) -> Socket {
		Socket::new(rx.map(Ok), ChannelSocketWriter { tx })
	}
}

/// A [`SocketWriter`] over an [`async_channel::Sender<Message>`]. Sends are
/// non-fatal (a closed peer channel is ignored, matching the tungstenite writer's
/// already-closed handling); a close pushes a [`Message::Close`] then closes the
/// sender so the peer's reader ends.
#[derive(Clone)]
struct ChannelSocketWriter {
	tx: Sender<Message>,
}

impl SocketWriter for ChannelSocketWriter {
	fn send_boxed(&mut self, msg: Message) -> BoxFuture<'static, Result<()>> {
		let tx = self.tx.clone();
		async move {
			tx.send(msg).await.ok();
			Ok(())
		}
		.boxed()
	}
	fn close_boxed(
		&mut self,
		close: Option<CloseFrame>,
	) -> BoxFuture<'static, Result<()>> {
		let tx = self.tx.clone();
		async move {
			tx.send(Message::Close(close)).await.ok();
			tx.close();
			Ok(())
		}
		.boxed()
	}
}

/// Registers the shared boot + teardown observers, mirroring [`SocketServer`] (see
/// [`ServerLifecycle`]).
fn on_add(mut world: DeferredWorld, cx: HookContext) {
	ServerLifecycle::<ChannelSocketServer>::add_observers(&mut world, cx.entity);
}

impl BootServer for ChannelSocketServer {
	const SELECTOR: &'static str = "channel";

	fn serve(
		entity: AsyncEntity,
		shutdown: OnceValueRx<()>,
	) -> LocalBoxedFuture<'static, Result> {
		Box::pin(start_channel_socket_server(entity, shutdown))
	}
}

/// The accept loop: adopt each connection delivered over the channel as a child
/// [`Socket`], exactly as the tungstenite accept path adopts an accepted WSS
/// connection. Parks like [`SocketServer`]; ends when the shutdown signal resolves
/// or the connections channel closes.
async fn start_channel_socket_server(
	entity: AsyncEntity,
	shutdown: OnceValueRx<()>,
) -> Result {
	if !entity.is_alive().await {
		return Ok(());
	}
	let connections = entity
		.get::<ChannelSocketServer, _>(|server| server.connections.clone())
		.await?;

	let accept = {
		let entity = entity.clone();
		async move {
			while let Ok(conn) = connections.recv().await {
				// build the server-side socket and adopt it as a child, wiring its
				// `MessageRecv` / `MessageSend` exactly like the tungstenite path.
				entity.spawn_child(Socket::channel(conn.tx, conn.rx)).await;
			}
			Result::Ok(())
		}
	};
	beet_core::exports::futures_lite::future::or(accept, async move {
		shutdown.wait().await;
		Result::Ok(())
	})
	.await
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::sockets::common_handlers::echo_message;

	/// Echo over the channel transport, the wasm-runnable port of the tungstenite
	/// `ecs_sockets` test: boot a `ChannelSocketServer` whose accepted sockets echo
	/// text, connect a client socket, send a message, then drive the app until the
	/// echo lands on the client (a bounded condition via
	/// [`AsyncRunner::poll_and_update`], not a settle). Runs on native and wasm.
	#[beet_core::test]
	async fn echoes_over_channel() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));
		let (server, client) = ChannelSocketServer::new();
		// the server echoes Text/Binary back on each accepted child socket
		let entity = app.world_mut().spawn(server).observe_any(echo_message).id();
		// boot through the fan-out (fire-and-forget: the call fans out and parks)
		app.world_mut().entity_mut(entity).run_async_local(
			|host| async move {
				host.call::<Boot, Response>(Boot::from(Request::get("/")))
					.await?;
				Ok(())
			},
		);

		// a client connection, used as a raw socket (not spawned into the ECS)
		let mut socket = client.connect().await.unwrap();
		socket.send(Message::text("ahoy")).await.unwrap();

		// drive the app until the server echoes the message back to the client
		let echo = AsyncRunner::poll_and_update(
			|| {
				app.update();
			},
			socket.next(),
		)
		.await;
		echo.unwrap().unwrap().xpect_eq(Message::text("ahoy"));
	}
}
