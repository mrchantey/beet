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
			.add_plugins(ActionPlugin::<InsertOnTrigger<OnUserMessage,Running>>::default());
	}
}

/// User messages received either internally or externally
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct OnUserMessage(pub String);
