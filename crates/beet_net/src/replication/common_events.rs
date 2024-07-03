use super::AppExtReplicate;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


/// Sent from this app, usually once assets are ready.
#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct AppReady;

pub struct CommonEventsPlugin;

impl Plugin for CommonEventsPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<AppReady>()
			.replicate_event_outgoing::<AppReady>()
			.add_plugins(ActionPlugin::<TriggerOnRun<AppReady>>::default());
		// .add_systems(Startup, ready);
	}
}


// fn ready(mut events: EventWriter<AppReady>) { events.send(AppReady); }
