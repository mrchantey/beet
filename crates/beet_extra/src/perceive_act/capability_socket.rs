//! Binds an agent's capability routes to the socket clients that serve them.
//!
//! The agent is a socket **server**. Each client that connects is asked its role
//! via a `whoami` request the server originates, and that role's capability routes
//! are rebound to forward over the connection ([`socket_exchange`]) instead of
//! handling locally. Throwaway glue for the perceive-act demo: the role -> paths
//! table ([`capability_routes`]) is hardcoded and deletable.
use crate::beet::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
// the socket surface (`Socket`, `ExchangeSocket`, `socket_exchange`, `SocketReady`,
// `ChannelSocketServer`) is a module in the net prelude, not a glob.
use beet_net::sockets::*;
use beet_router::prelude::*;

/// Marker on the agent's socket-server host. Each connection it accepts is asked for
/// its role via `whoami`, and that role's capability routes are bound to forward over
/// the connection.
#[derive(Debug, Default, Clone, Component, Reflect)]
#[reflect(Component, Default)]
pub struct CapabilityServer;

/// The role a client serves ("head"/"body"), on the client's socket root. Read by `whoami`.
#[derive(Debug, Clone, Component, Reflect, Deref)]
#[reflect(Component)]
pub struct ClientRole(pub SmolStr);

/// Marker inserted on an agent route once it is bound to a connection (for observability/tests).
#[derive(Debug, Clone, Component, Reflect)]
#[reflect(Component)]
pub struct CapabilityBound;

/// Announce which role this client serves, so the server can bind the matching
/// capabilities.
#[action(route = "whoami")]
#[derive(Component, Reflect)]
#[reflect(Component)]
pub async fn WhoAmI(cx: ActionContext<()>) -> Result<String> {
	cx.caller
		.with_state::<AncestorQuery<&ClientRole>, _>(|entity, roles| {
			roles.get(entity).map(|role| role.0.to_string())
		})
		.await?
}

/// The hardcoded role -> capability-path table wiring the perceive-act demo. Pure
/// glue: delete it (and the socket machinery around it) once roles are declared in
/// markup.
fn capability_routes(role: &str) -> &'static [&'static str] {
	match role {
		"head" => &["take-photo", "speak-text", "set-emotion"],
		"body" => &["apply-heading"],
		_ => &[],
	}
}

/// Bind a freshly-accepted connection to its role's capabilities.
///
/// [`SocketReady`] fires on every wired [`Socket`], including a client the agent's
/// [`ChannelSocketServer`] adopts as a child; only those children (parent is a
/// [`CapabilityServer`]) are bound, off the observer on the connection's own thread.
fn bind_on_connection_ready(
	ev: On<SocketReady>,
	parents: Query<&ChildOf>,
	servers: Query<(), With<CapabilityServer>>,
	commands: AsyncCommands,
) {
	let connection = ev.target();
	// only the agent server's own accepted connections, not other ready sockets.
	let Some(server) = parents
		.get(connection)
		.map(ChildOf::parent)
		.ok()
		.filter(|parent| servers.contains(*parent))
	else {
		return;
	};
	commands.entity(connection).run_local(async move |connection| {
		if let Err(err) = bind_connection(connection, server).await {
			warn!("failed to bind capability connection: {err}");
		}
	});
}

/// Handshake a connection and rebind its role's routes to forward over it.
async fn bind_connection(connection: AsyncEntity, server: Entity) -> Result {
	// enable the duplex exchange so the server can originate `whoami` and the bound
	// routes can forward over this connection. `json` (not `postcard`) so the demo
	// needs no extra feature over `thread`, and frames stay debuggable.
	connection.insert(ExchangeSocket::json()).await?;
	// originate `whoami` to the freshly-connected client and read its role.
	let response = connection
		.call_detached(
			socket_exchange(connection.id()),
			Request::get("whoami")
				.with_header::<header::Accept>(MediaType::Json),
		)
		.await?;
	let body = response.into_result().await?.body.into_string().await?;
	// the client serves `whoami` as a string; strip any json quoting.
	let role = body.trim().trim_matches('"').to_string();
	// resolve the agent's route entities for this role and forward them over the socket.
	// the thread (a `CreateThread` boot) starts in parallel; the first capability call
	// is preceded by a model round-trip, so this fast local handshake wins the race, and
	// `take-photo` falls back to its local handler if a photo is somehow needed first.
	let targets = connection
		.world()
		.with(move |world| capability_route_entities(world, server, &role))
		.await;
	for target in targets {
		connection
			.world()
			.entity(target)
			.insert((socket_exchange(connection.id()), CapabilityBound))
			.await?;
	}
	Ok(())
}

/// The agent route entities matching `role`'s capabilities, found in the server
/// root's [`RouteTree`]. Scoped to the server's own root, so a separate root's
/// identically-named routes are never touched.
fn capability_route_entities(
	world: &mut World,
	server: Entity,
	role: &str,
) -> Vec<Entity> {
	world.with_state::<(Query<&ChildOf>, Query<&RouteTree>), _>(
		|(ancestors, trees)| {
			let root = ancestors.root_ancestor(server);
			let Ok(tree) = trees.get(root) else {
				return Vec::new();
			};
			capability_routes(role)
				.iter()
				.filter_map(|path| {
					tree.find(&Request::get(*path).path().clone())
						.map(|node| node.entity)
				})
				.collect()
		},
	)
}

/// Registers the capability-binding types and the connection-ready observer.
pub struct CapabilityBindingPlugin;

impl Plugin for CapabilityBindingPlugin {
	fn build(&self, app: &mut App) {
		app.register_type::<CapabilityServer>()
			.register_type::<ClientRole>()
			.register_type::<CapabilityBound>()
			.register_type::<WhoAmI>()
			.add_observer(bind_on_connection_ready);
	}
}

#[cfg(test)]
mod test {
	use super::*;
	// the head capability the test forwards: `SetEmotion` and its `Emotion`, siblings
	// under `perceive_act`.
	use crate::perceive_act::Emotion;
	use crate::perceive_act::SetEmotion;
	use crate::perceive_act::SetEmotionInput;

	/// The whole core: the agent (a socket server) accepts a mock head client,
	/// originates `whoami`, binds `set-emotion` to the connection, and a
	/// `set-emotion` call on the agent forwards over the socket to the head, which
	/// serves it and records the [`Emotion`].
	#[beet_core::test]
	async fn binds_role_and_forwards_call() {
		let mut app = App::new();
		// `ThreadPlugin` pulls `RouterPlugin` (route trees) + `AsyncPlugin` + `NetPlugin`.
		app.add_plugins((MinimalPlugins, ThreadPlugin::default()))
			.add_plugins(CapabilityBindingPlugin);

		let (server, client) = ChannelSocketServer::new();
		// the agent root: its `set-emotion` child joins the root's route tree.
		let agent = app
			.world_mut()
			.spawn((CapabilityServer, server, Router))
			.id();
		// track the `set-emotion` route entity so the test can await its binding.
		let agent_emotion =
			app.world_mut().spawn((SetEmotion, ChildOf(agent))).id();
		app.world_mut().flush();
		// boot the agent's socket server through the fan-out (parks on its keep-alive).
		app.world_mut()
			.entity_mut(agent)
			.run_async_local(|host| async move {
				host.call::<Boot, Response>(Boot::from(Request::get("/")))
					.await?;
				Ok(())
			});

		// the mock head client: a SEPARATE root, so its identically-named routes live
		// in their own route tree (proving per-root isolation).
		let head_socket = client.connect().await.unwrap();
		let head = app
			.world_mut()
			.spawn((
				head_socket,
				ExchangeSocket::json(),
				Router,
				ClientRole(SmolStr::new("head")),
			))
			.id();
		app.world_mut().spawn((WhoAmI, ChildOf(head)));
		// track the head's `set-emotion` route entity to assert the forwarded call.
		let head_emotion =
			app.world_mut().spawn((SetEmotion, ChildOf(head))).id();
		app.world_mut().flush();

		// drive until the agent's `set-emotion` route is bound to the connection.
		app_ext::update_until(&mut app, |world| {
			world.get::<CapabilityBound>(agent_emotion).is_some()
		})
		.await
		.xpect_true();

		// call `set-emotion` on the agent; it forwards over the socket to the head.
		app.world_mut()
			.entity_mut(agent)
			.run_async_local(|agent| async move {
				agent
					.call_detached(
						route_action(),
						Request::get("set-emotion")
							.with_body(serde_json::to_string(&SetEmotionInput {
								emotion: Emotion::Angry,
							})?)
							.with_header::<header::ContentType>(MediaType::Json),
					)
					.await?;
				Ok(())
			});

		// drive until the mock head has recorded the forwarded emotion.
		app_ext::update_until(&mut app, |world| {
			world.get::<Emotion>(head_emotion).is_some()
		})
		.await
		.xpect_true();

		app.world_mut()
			.get::<Emotion>(head_emotion)
			.copied()
			.xpect_eq(Some(Emotion::Angry));
	}
}
