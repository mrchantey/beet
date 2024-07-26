use beetmash::prelude::*;
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

		#[cfg(feature = "tokio")]
		const DEFAULT_SOCKET_URL: &str = "ws://127.0.0.1:3000/ws";

		#[cfg(target_arch = "wasm32")]
		app.add_transport(WebEventClient::new_with_window());

		#[cfg(feature = "tokio")]
		app.add_transport(NativeWsClient::new(DEFAULT_SOCKET_URL).unwrap());
	}
}

/// Placeholder, this is the workflow for extending replicate
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct SomeCustomEvent(pub String);
