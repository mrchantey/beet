//! The server-to-client websocket channel, see the [module docs](super).

use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_net::sockets::*;
// explicit: `sockets::Message` must win over bevy's `Message` trait in the
// preludes
use beet_net::sockets::Message;

/// The main-port path a browser upgrades on to join the [`ClientIo`] channel.
pub const CLIENT_IO_PATH: &str = "__client_io";

/// A general purpose server-to-client websocket channel.
///
/// Each connected client becomes a child [`Socket`](beet_net::prelude::Socket)
/// entity, so the channel's children are its live client registry. Trigger
/// [`ClientIoBroadcast`] on this entity to message every client, eg the
/// [`LiveReload`](super::LiveReload) watcher's `reload`.
///
/// The channel rides the main HTTP port: browsers upgrade at [`CLIENT_IO_PATH`]
/// (the [`client_io_route`] `default_router` wires in), the backend lands the
/// upgraded connection as a [`Socket`] and fires [`OnWebSocketUpgrade`], and
/// [`adopt_client_io_socket`] re-parents it under this channel.
#[derive(Debug, Default, Clone, Component)]
pub struct ClientIo;

/// The `/__client_io` route: a [`WebSocketUpgrade`] handler `default_router`
/// wires in under the `client_io` feature, so every HTTP router exposes the
/// upgrade endpoint on its own port.
pub fn client_io_route() -> impl Bundle {
	exchange_route("__client_io", exchange_handler(client_io_upgrade))
}

/// Handler: upgrade an incoming `/__client_io` request to a WebSocket.
fn client_io_upgrade(cx: ActionContext<Request>) -> Response {
	WebSocketUpgrade::from_request(&cx).into()
}

/// Broadcasts a [`Message`] to every connected client of the target
/// [`ClientIo`] channel.
#[derive(Debug, Clone, EntityTargetEvent)]
pub struct ClientIoBroadcast(pub Message);

/// Observer: adopt a [`Socket`] the backend upgraded (via [`client_io_route`])
/// into the [`ClientIo`] channel, re-parenting it so [`broadcast_to_clients`]
/// reaches it. Despawns the orphan socket when no channel exists.
pub(crate) fn adopt_client_io_socket(
	ev: On<OnWebSocketUpgrade>,
	channels: Query<Entity, With<ClientIo>>,
	mut commands: Commands,
) {
	match channels.iter().next() {
		Some(channel) => {
			commands.entity(ev.event().socket).insert(ChildOf(channel));
		}
		// no channel to adopt into: drop the connection rather than leak it
		None => commands.entity(ev.event().socket).despawn(),
	}
}

/// Observer: fan a [`ClientIoBroadcast`] out to the channel's connected
/// clients, ie its child [`Socket`](beet_net::prelude::Socket) entities.
pub(crate) fn broadcast_to_clients(
	ev: On<ClientIoBroadcast>,
	children: Query<&Children>,
	mut commands: Commands,
) {
	for client in children
		.get(ev.target())
		.into_iter()
		.flat_map(|children| children.iter())
	{
		commands
			.entity(client)
			.trigger_target(MessageSend(ev.event().0.clone()));
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A child entity recording every [`MessageSend`] it receives.
	fn client_captor(
		world: &mut World,
		channel: Entity,
	) -> Store<Vec<Message>> {
		let received = Store::<Vec<Message>>::default();
		let captor = received.clone();
		world.spawn(ChildOf(channel)).observe_any(
			move |ev: On<MessageSend>| {
				captor.push(ev.event().inner().clone());
			},
		);
		received
	}

	#[beet_core::test]
	fn broadcasts_to_every_client() {
		let mut world = World::new();
		world.add_observer(broadcast_to_clients);
		// no listener required: the fan-out is independent of a live socket
		let channel = world.spawn(ClientIo).id();
		let client_one = client_captor(&mut world, channel);
		let client_two = client_captor(&mut world, channel);
		// a stranger outside the channel receives nothing
		let other = world.spawn_empty().id();
		let stranger = client_captor(&mut world, other);

		world
			.entity_mut(channel)
			.trigger_target(ClientIoBroadcast(Message::text("reload")));
		world.flush();

		client_one.get().xpect_eq(vec![Message::text("reload")]);
		client_two.get().xpect_eq(vec![Message::text("reload")]);
		stranger.get().xpect_eq(Vec::<Message>::new());
	}

	#[beet_core::test]
	fn adopts_an_upgraded_socket_into_the_channel() {
		let mut world = World::new();
		world.add_observer(adopt_client_io_socket);
		let channel = world.spawn(ClientIo).id();
		// stand in for the backend's landed `Socket` entity
		let socket = world.spawn_empty().id();
		world.trigger(OnWebSocketUpgrade { socket });
		world.flush();
		// the socket is now a child of the channel, ie part of its registry
		world
			.entity(socket)
			.get::<ChildOf>()
			.unwrap()
			.parent()
			.xpect_eq(channel);
	}

	#[beet_core::test]
	fn despawns_an_orphan_socket_when_no_channel() {
		let mut world = World::new();
		world.add_observer(adopt_client_io_socket);
		let socket = world.spawn_empty().id();
		world.trigger(OnWebSocketUpgrade { socket });
		world.flush();
		world.get_entity(socket).is_err().xpect_true();
	}

	/// End to end over the main HTTP port: a browser-like client upgrades at
	/// `/__client_io` (wired by `default_router`), is adopted into the channel,
	/// and receives a [`ClientIoBroadcast`]. This is the side-port replacement.
	#[beet_core::test]
	async fn broadcasts_over_the_upgraded_main_port() {
		// reuse the mini server's pre-bound test listener so there is no port
		// race; spawn it on a bare router (no `HttpServer`, so the `Server`
		// orchestrator does not also try to bind the same port).
		let (server, on_spawn) =
			HttpServer::new_test(start_mini_http_server_with_tcp);
		let port = server.port.unwrap();

		std::thread::spawn(move || {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, ServerPlugin, RouterPlugin));
			// the router (wires `/__client_io`), the channel, and the listener
			app.world_mut()
				.spawn((default_router(), ClientIo, on_spawn));
			// once a client is adopted, broadcast `reload` to the channel each
			// frame (the client breaks after the first message)
			app.add_systems(
				bevy::app::Update,
				|channels: Query<(Entity, &Children), With<ClientIo>>,
				 mut commands: Commands| {
					for (channel, _children) in
						channels.iter().filter(|(_, kids)| !kids.is_empty())
					{
						commands.entity(channel).trigger_target(
							ClientIoBroadcast(Message::text(RELOAD_MESSAGE)),
						);
					}
				},
			);
			app.run();
		});
		time_ext::sleep_millis(200).await;

		// the browser client connects to the channel over the main port
		let mut client =
			Socket::connect(format!("ws://127.0.0.1:{port}/__client_io"))
				.await
				.unwrap();

		// the server fans `reload` out over the upgraded channel
		let mut received = None;
		for _ in 0..40 {
			if let Some(Ok(Message::Text(text))) = client.next().await {
				received = Some(text);
				break;
			}
		}
		received.xpect_eq(Some(RELOAD_MESSAGE.to_string()));
		client.close(None).await.ok();
	}
}
