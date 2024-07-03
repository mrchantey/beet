use beet::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub struct ExampleReplicatePlugin;

impl Plugin for ExampleReplicatePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ReplicatePlugin, CommonEventsPlugin))
			.add_event::<OnUserMessage>()
			.replicate_event_incoming::<OnUserMessage>();
	}
}


#[derive(Event, Deref, DerefMut, Serialize, Deserialize)]
pub struct OnUserMessage(pub String);
