use crate::lightyear::*;
use bevy::prelude::*;
use lightyear::prelude::client::*;
use lightyear::prelude::*;

pub struct ExampleClientPlugin;

impl Plugin for ExampleClientPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Startup, init);
		app.add_systems(PreUpdate, handle_connection.after(MainSet::Receive));
		app.add_systems(Update, (send_message, receive_message));
	}
}

/// Startup system for the client
pub(crate) fn init(mut commands: Commands) { commands.connect_client(); }

/// Listen for events to know when the client is connected;
/// - spawn a text entity to display the client id
/// - spawn a client-owned cursor entity that will be replicated to the server
pub(crate) fn handle_connection(
	mut connection_event: EventReader<ConnectEvent>,
) {
	for event in connection_event.read() {
		let client_id = event.client_id();
		log::info!("Connected width id {client_id}");
	}
}

// System to receive messages on the client
pub(crate) fn receive_message(
	mut reader: EventReader<MessageEvent<BeetMessage>>,
) {
	for event in reader.read() {
		info!("Received message: {:?}", event.message());
	}
}

/// Send messages from server to clients
pub(crate) fn send_message(
	mut client: ResMut<ConnectionManager>,
	// input: Res<ButtonInput<KeyCode>>,
) {
	// if input.pressed(KeyCode::KeyM) {
	let message = BeetMessage("this is a message".into());
	// info!("Send message: {:?}", message);
	// the message will be re-broadcasted by the server to all clients
	client
		.send_message_to_target::<BeetChannel, BeetMessage>(
			&message,
			NetworkTarget::All,
		)
		.unwrap_or_else(|e| {
			error!("Failed to send message: {:?}", e);
		});
	// }
}
