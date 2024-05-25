use super::AppExtReplicate;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;



#[derive(Debug, Clone, Serialize, Deserialize, Event)]
pub struct AppReady;

pub struct CommonEventsPlugin;

impl Plugin for CommonEventsPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<AppReady>()
			.replicate_event_outgoing::<AppReady>()
			.add_systems(Startup, ready);
	}
}


fn ready(mut events: EventWriter<AppReady>) { events.send(AppReady); }
