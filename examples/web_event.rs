use beet::prelude::*;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;


#[derive(Debug, Event, Serialize, Deserialize)]
pub struct MyEvent(pub String);


struct Foo(pub Arc<Mutex<WebEventClient>>);

impl Plugin for Foo {
	fn build(&self, app: &mut App) { todo!() }
}

fn main() {
	let mut app = App::new();
	// app.add_plugins();
	// app.add_plugins(TransportPlugin::arc(WebWsClient::new("").unwrap()));
	app.add_plugins(
		TransportPlugin::closure(WebEventClient::new_with_window()),
	);
	app.add_plugins((
		// MinimalPlugins,
		// LogPlugin::default(),
		// ReplicatePlugin,
		// TransportPlugin::arc(WebEventClient::new_with_window()),
	))
	.add_event::<MyEvent>()
	.replicate_event::<MyEvent>()
	.add_systems(Update, handle_event);
}


fn handle_event(mut events: EventReader<MyEvent>) {
	for event in events.read() {
		log::info!("Received event: {:?}", event);
	}
}
