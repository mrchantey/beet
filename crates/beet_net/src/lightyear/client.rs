//! The client plugin.
//! The client will be responsible for:
//! - connecting to the server at Startup
//! - sending inputs to the server
//! - applying inputs to the locally predicted player (for prediction to work, inputs have to be applied to both the
//! predicted entity and the server entity)
use crate::lightyear::protocol::Direction;
use crate::lightyear::protocol::*;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(PreUpdate, handle_connection.after(MainSet::Receive));
		// Inputs have to be buffered in the FixedPreUpdate schedule
		app.add_systems(
			FixedPreUpdate,
			buffer_input.in_set(InputSystemSet::BufferInputs),
		);
		app.add_systems(
			Update,
			(
				receive_message1,
				receive_entity_spawn,
				receive_entity_despawn,
				receive_player_id_insert,
				handle_predicted_spawn,
				handle_interpolated_spawn,
			),
		);
		app.add_systems(OnEnter(NetworkingState::Disconnected), on_disconnect);
	}
}

/// Component to identify the text displaying the client id
#[derive(Component)]
pub struct ClientIdText;

/// Listen for events to know when the client is connected, and spawn a text entity
/// to display the client id
pub(crate) fn handle_connection(
	mut connection_event: EventReader<ConnectEvent>,
) {
	for event in connection_event.read() {
		let client_id = event.client_id();
		log::info!("Connected to server with client id: {}", client_id);
	}
}

/// System that reads from peripherals and adds inputs to the buffer
/// This system must be run in the
pub(crate) fn buffer_input(
	tick_manager: Res<TickManager>,
	mut input_manager: ResMut<InputManager<Inputs>>,
	keypress: Res<ButtonInput<KeyCode>>,
) {
	let tick = tick_manager.tick();
	let mut input = Inputs::None;
	let mut direction = Direction {
		up: false,
		down: false,
		left: false,
		right: false,
	};
	if keypress.pressed(KeyCode::KeyW) || keypress.pressed(KeyCode::ArrowUp) {
		direction.up = true;
	}
	if keypress.pressed(KeyCode::KeyS) || keypress.pressed(KeyCode::ArrowDown) {
		direction.down = true;
	}
	if keypress.pressed(KeyCode::KeyA) || keypress.pressed(KeyCode::ArrowLeft) {
		direction.left = true;
	}
	if keypress.pressed(KeyCode::KeyD) || keypress.pressed(KeyCode::ArrowRight)
	{
		direction.right = true;
	}
	if !direction.is_none() {
		input = Inputs::Direction(direction);
	}
	if keypress.pressed(KeyCode::Backspace) {
		input = Inputs::Delete;
	}
	if keypress.pressed(KeyCode::Space) {
		input = Inputs::Spawn;
	}
	input_manager.add_input(input, tick)
}

/// System to receive messages on the client
pub(crate) fn receive_message1(
	mut reader: EventReader<MessageEvent<Message1>>,
) {
	for event in reader.read() {
		info!("Received message: {:?}", event.message());
	}
}

/// Example system to handle EntitySpawn events
pub(crate) fn receive_entity_spawn(mut reader: EventReader<EntitySpawnEvent>) {
	for event in reader.read() {
		info!("Received entity spawn: {:?}", event.entity());
	}
}

/// Example system to handle EntitySpawn events
pub(crate) fn receive_entity_despawn(
	mut reader: EventReader<EntityDespawnEvent>,
) {
	for event in reader.read() {
		info!("Received entity despawn: {:?}", event.entity());
	}
}

/// Example system to handle ComponentInsertEvent events
pub(crate) fn receive_player_id_insert(
	mut reader: EventReader<ComponentInsertEvent<PlayerId>>,
) {
	for event in reader.read() {
		info!(
			"Received component PlayerId insert for entity: {:?}",
			event.entity()
		);
	}
}

pub(crate) fn handle_predicted_spawn(
	mut predicted: Query<Entity, Added<Predicted>>,
) {
	for entity in predicted.iter_mut() {
		log::info!("Predicted entity spawned: {:?}", entity);
	}
}

/// When the predicted copy of the client-owned entity is spawned, do stuff
/// - assign it a different saturation
/// - keep track of it in the Global resource
pub(crate) fn handle_interpolated_spawn(
	mut interpolated: Query<Entity, Added<Interpolated>>,
) {
	for entity in interpolated.iter_mut() {
		log::info!("Interpolated entity spawned: {:?}", entity);
	}
}


/// Remove all entities when the client disconnect
fn on_disconnect(
	mut commands: Commands,
	player_entities: Query<Entity, With<PlayerId>>,
	debug_text: Query<Entity, With<ClientIdText>>,
) {
	for entity in player_entities.iter() {
		commands.entity(entity).despawn_recursive();
	}
	for entity in debug_text.iter() {
		commands.entity(entity).despawn_recursive();
	}
}
