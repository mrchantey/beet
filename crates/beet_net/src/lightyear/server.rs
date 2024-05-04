//! The server side of the example.
//! It is possible (and recommended) to run the server in headless mode (without any rendering plugins).
//!
//! The server will:
//! - spawn a new player entity for each client that connects
//! - read inputs from the clients and move the player entities accordingly
//!
//! Lightyear will handle the replication of entities automatically if you add a `Replicate` component to them.
use crate::lightyear::protocol::*;
use bevy::prelude::*;
use lightyear::prelude::server::*;
use lightyear::prelude::*;
use std::collections::HashMap;

pub struct ExampleServerPlugin;

impl Plugin for ExampleServerPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(Global {
			client_id_to_entity_id: Default::default(),
		});
		app.add_systems(Startup, start_server);
		// the physics/FixedUpdates systems that consume inputs should be run in this set
		app.add_systems(Update, (send_message, handle_connections));
	}
}

#[derive(Resource)]
pub(crate) struct Global {
	pub client_id_to_entity_id: HashMap<ClientId, Entity>,
}

/// Start the server
fn start_server(mut commands: Commands) { commands.start_server(); }

/// Server connection system, create a player upon connection
pub(crate) fn handle_connections(
	mut connections: EventReader<ConnectEvent>,
	mut disconnections: EventReader<DisconnectEvent>,
	mut global: ResMut<Global>,
	mut commands: Commands,
) {
	for connection in connections.read() {
		let client_id = *connection.context();
		// server and client are running in the same app, no need to replicate to the local client
		let replicate = Replicate {
			prediction_target: NetworkTarget::Single(client_id),
			interpolation_target: NetworkTarget::AllExceptSingle(client_id),
			..default()
		};
		let entity = commands.spawn((PlayerBundle::new(client_id), replicate));
		// Add a mapping from client id to entity id
		global.client_id_to_entity_id.insert(client_id, entity.id());
		info!("Create entity {:?} for client {:?}", entity.id(), client_id);
	}
	for disconnection in disconnections.read() {
		let client_id = disconnection.context();
		// TODO: handle this automatically in lightyear
		//  - provide a Owned component in lightyear that can specify that an entity is owned by a specific player?
		//  - maybe have the client-id to entity-mapping in the global metadata?
		//  - despawn automatically those entities when the client disconnects
		if let Some(entity) = global.client_id_to_entity_id.remove(client_id) {
			if let Some(mut entity) = commands.get_entity(entity) {
				entity.despawn();
			}
		}
	}
}

/// Send messages from server to clients (only in non-headless mode, because otherwise we run with minimal plugins
/// and cannot do input handling)
pub(crate) fn send_message(
	mut server: ResMut<ConnectionManager>,
	input: Option<Res<ButtonInput<KeyCode>>>,
) {
	if input.is_some_and(|input| input.pressed(KeyCode::KeyM)) {
		let message = Message1(5);
		info!("Send message: {:?}", message);
		server
			.send_message_to_target::<Channel1, Message1>(
				&Message1(5),
				NetworkTarget::All,
			)
			.unwrap_or_else(|e| {
				error!("Failed to send message: {:?}", e);
			});
	}
}
