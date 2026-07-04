//! [`PersistentSocket`]: a client [`Socket`] that keeps itself connected.
use crate::prelude::Url;
use crate::sockets::*;
use beet_core::prelude::*;

/// Keeps a client [`Socket`] connected to `url` for the life of the entity.
///
/// Dials with exponential backoff until the server is reachable (a scene often
/// loads before its server is up), inserts the connected [`Socket`], then waits
/// for [`SocketClosed`] and dials again, so a client survives a server restart
/// without reloading the scene. Each (re)connection triggers [`SocketReady`],
/// re-running whatever wiring listens for it (eg a server's `whoami` binding).
///
/// The default policy dials immediately, then backs off 250ms doubling to a
/// 10s ceiling, forever.
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
#[component(on_add = on_add)]
pub struct PersistentSocket {
	/// The socket url to keep connected to, eg `ws://192.168.1.7:8338`.
	/// [`Url`] carries no `Reflect` impl (its params ride a `MultiMap`), so the
	/// field is reflect-opaque; construct via [`Self::new`].
	#[reflect(ignore)]
	pub url: Url,
	/// The redial policy: how long to wait between failed connection attempts.
	#[reflect(ignore, default = "PersistentSocket::default_backoff")]
	pub backoff: Backoff,
}

impl PersistentSocket {
	/// A persistent socket to `url` with the default redial policy.
	pub fn new(url: impl Into<Url>) -> Self {
		Self {
			url: url.into(),
			backoff: Self::default_backoff(),
		}
	}

	/// 250ms doubling to a 10s ceiling, forever.
	fn default_backoff() -> Backoff {
		Backoff::new(
			u32::MAX,
			Duration::from_millis(250),
			Duration::from_secs(10),
		)
	}

	/// Override the redial policy.
	pub fn with_backoff(mut self, backoff: Backoff) -> Self {
		self.backoff = backoff;
		self
	}
}

fn on_add(mut world: DeferredWorld, cx: HookContext) {
	world
		.commands()
		.entity(cx.entity)
		.queue_async_local(connection_loop);
}

/// The connection loop: dial until connected, park until closed, repeat. Ends
/// when the entity (or its [`PersistentSocket`]) goes away.
async fn connection_loop(entity: AsyncEntity) -> Result {
	let config = entity.get_cloned::<PersistentSocket>().await?;
	// the reader task signals every connection end into this channel
	let (closed_send, closed_recv) = writer_channel::unbounded::<()>();
	entity.insert(SocketClosedNotify(closed_send)).await?;
	loop {
		// dial until connected, backing off to the ceiling
		let mut frames = config.backoff.iter();
		let socket = loop {
			match Socket::connect(config.url.to_string()).await {
				Ok(socket) => break socket,
				Err(err) => {
					let delay = frames
						.next()
						.and_then(|frame| frame.next_attempt)
						.ok_or_else(|| {
							bevyhow!(
								"socket connect to {} gave up: {err}",
								config.url
							)
						})?;
					debug!(
						"socket connect to {} failed ({err}), retrying in {delay:?}",
						config.url
					);
					time_ext::sleep(delay).await;
				}
			}
		};
		info!("socket connected: {}", config.url);
		entity.insert(socket).await?;
		// park until the connection dies, then go round again. The brief sleep
		// damps a hot loop against a server that accepts then instantly drops.
		closed_recv
			.recv()
			.await
			.ok_or_else(|| bevyhow!("socket closed channel dropped"))?;
		warn!("socket closed: {}, reconnecting", config.url);
		time_ext::sleep(*config.backoff.min()).await;
	}
}

#[cfg(all(test, feature = "tungstenite", not(target_arch = "wasm32")))]
mod test {
	use super::*;
	use crate::sockets::Message;
	use crate::sockets::echo_socket_server::EchoSocketServer;

	/// A writer whose sends vanish, for the doomed stand-in socket below.
	struct NoopWriter;
	impl SocketWriter for NoopWriter {
		fn send_boxed(&mut self, _msg: Message) -> SendBoxedFuture<Result<()>> {
			Box::pin(async { Ok(()) })
		}
		fn close_boxed(
			&mut self,
			_close: Option<CloseFrame>,
		) -> SendBoxedFuture<Result<()>> {
			Box::pin(async { Ok(()) })
		}
	}

	/// The persistent socket dials, survives its connection dying (the
	/// scenario: a server restart severs the transport), and redials: two
	/// `SocketReady` firings against one echo server. After the reconnect a
	/// single send echoes back exactly once, proving the dead connection's
	/// writer was torn down rather than double-sending (no per-connection leak).
	#[beet_core::test]
	async fn reconnects_after_close() {
		let server = EchoSocketServer::new().await;
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AsyncPlugin));
		let ready_count = Store::<Vec<()>>::default();
		let echoes = Store::<Vec<Message>>::default();
		let client = app
			.world_mut()
			.spawn(PersistentSocket::new(server.url()))
			.observe_any(move |_ev: On<SocketReady>| {
				ready_count.push(());
			})
			.observe_any(move |ev: On<MessageRecv>| {
				if let Message::Text(_) = ev.event().inner() {
					echoes.push(ev.event().inner().clone());
				}
			})
			.id();

		// first connection
		app_ext::update_until(&mut app, move |_| ready_count.len() == 1)
			.await
			.xpect_true();

		// sever the connection: swap in a socket whose transport errors
		// immediately (what a dropped peer looks like to the reader), so the
		// reader-task cleanup -> closed-notify -> redial chain runs for real.
		// Real time passes before the redial (the damping sleep), so the
		// frame-capped `update_until` would burn out first.
		app.world_mut().entity_mut(client).insert(Socket::new(
			futures::stream::once(async {
				Err::<Message, _>(bevyhow!("peer dropped"))
			}),
			NoopWriter,
		));
		app_ext::update_until_timeout(
			&mut app,
			move |_| ready_count.len() >= 3,
			Duration::from_secs(3),
		)
		.await
		.xpect_true();

		// one send over the new connection echoes back exactly once
		app.world_mut()
			.entity_mut(client)
			.trigger_target(MessageSend(Message::text("still here")));
		app_ext::update_until_timeout(
			&mut app,
			move |_| !echoes.is_empty(),
			Duration::from_secs(1),
		)
		.await
		.xpect_true();
		// settle a few frames: a leaked writer would surface a duplicate echo
		for _ in 0..10 {
			app.update();
			AsyncRunner::tick().await;
		}
		echoes.len().xpect_eq(1usize);
	}
}

