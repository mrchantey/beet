use beet::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub struct ExampleReplicatePlugin;

impl Plugin for ExampleReplicatePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ReplicatePlugin, CommonEventsPlugin))
			.add_event::<OnUserMessage>()
			.replicate_event_incoming::<OnUserMessage>()
			.add_event::<AppLoaded>()
			.replicate_event_outgoing::<AppLoaded>()
			.add_plugins(ActionPlugin::<TriggerOnRun<AppLoaded>>::default());
	}
}


/// User messages received either internally or externally
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct OnUserMessage(pub String);

/// Sent from this bevy app to web ui etc to notify that assets etc have loaded.
#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct AppLoaded;
