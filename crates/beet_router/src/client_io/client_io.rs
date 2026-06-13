//! The server-to-client websocket channel, see the [module docs](super).

use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_net::sockets::*;
// explicit: `sockets::Message` must win over bevy's `Message` trait in the
// preludes
use beet_net::sockets::Message;

/// A general purpose server-to-client websocket channel.
///
/// Spawning one starts a [`SocketServer`] on [`Self::port`] (see
/// [`start_client_io`]); each connected client becomes a child
/// [`Socket`](beet_net::prelude::Socket) entity, so the channel's children are
/// its live client registry. Trigger [`ClientIoBroadcast`] on this entity to
/// message every client, eg the [`LiveReload`](super::LiveReload) watcher's
/// `reload`.
#[derive(Debug, Clone, Component)]
pub struct ClientIo {
	/// The websocket port. `None` means the OS will assign a port,
	/// which only a same-world consumer can discover, so dev hosts keep the
	/// default and the [`LiveReloadScript`](super::LiveReloadScript) widget
	/// can address it.
	pub port: Option<u16>,
}

impl Default for ClientIo {
	fn default() -> Self {
		Self {
			port: Some(DEFAULT_SOCKET_PORT),
		}
	}
}

/// Broadcasts a [`Message`] to every connected client of the target
/// [`ClientIo`] channel.
#[derive(Debug, Clone, EntityTargetEvent)]
pub struct ClientIoBroadcast(pub Message);

/// Observer: start the websocket listener for a spawned [`ClientIo`].
pub(crate) fn start_client_io(
	ev: On<Insert, ClientIo>,
	channels: Query<&ClientIo>,
	mut commands: Commands,
) -> Result {
	let port = channels.get(ev.entity)?.port;
	commands.entity(ev.entity).insert(SocketServer { port });
	Ok(())
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
	fn client_captor(world: &mut World, channel: Entity) -> Store<Vec<Message>> {
		let received = Store::<Vec<Message>>::default();
		let captor = received.clone();
		world
			.spawn(ChildOf(channel))
			.observe_any(move |ev: On<MessageSend>| {
				captor.push(ev.event().inner().clone());
			});
		received
	}

	#[beet_core::test]
	fn broadcasts_to_every_client() {
		let mut world = World::new();
		world.add_observer(broadcast_to_clients);
		// no `start_client_io` registered: the fan-out is independent of a
		// live listener
		let channel = world.spawn(ClientIo { port: None }).id();
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
}
