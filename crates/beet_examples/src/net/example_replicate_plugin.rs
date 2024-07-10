use crate::beet::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;

pub struct ExampleReplicatePlugin;

impl Plugin for ExampleReplicatePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((ReplicatePlugin, CommonEventsPlugin))
			.add_event::<SomeCustomEvent>()
			.replicate_event_incoming::<SomeCustomEvent>()
			/*-*/;
	}
}

/// Placeholder, this is the workflow for extending replicate
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct SomeCustomEvent(pub String);
