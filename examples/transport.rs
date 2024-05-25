use beet::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


#[derive(Debug, Event, Serialize, Deserialize)]
pub struct MyEvent(pub String);

fn main() {
	// app.add_plugins();
	// app.add_plugins(TransportPlugin::arc(WebWsClient::new("").unwrap()));
	App::new()
		.add_transport(DebugSendTransport)
		.add_transport(WebEventClient::new_with_window())
		.add_plugins((MinimalPlugins, LogPlugin::default(), ReplicatePlugin))
		.add_event::<MyEvent>()
		.replicate_event::<MyEvent>()
		// .add_systems(Startup, transmit)
		.add_systems(Update, handle_event)
		.run();
}

fn handle_event(mut events: EventReader<MyEvent>) {
	for event in events.read() {
		log::info!("Received event: {:?}", event);
	}
}
