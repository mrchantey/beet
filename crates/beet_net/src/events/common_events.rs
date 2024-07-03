use beet_ecs::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;
use crate::prelude::*;


/** 
 * This adds common replication events to the app.
 * It should be added before any other events in order
 * to preserve the registration ids:
 * 
 * - `0`: AppReady
 * - `1`: SpawnScene
*/
pub struct CommonEventsPlugin;

impl Plugin for CommonEventsPlugin {
	fn build(&self, app: &mut App) {
		app.add_event::<AppReady>()
			.replicate_event_outgoing::<AppReady>()
			.add_plugins(ActionPlugin::<TriggerOnRun<AppReady>>::default())
			.add_event::<OnSpawnScene>()
			.replicate_event_incoming::<OnSpawnScene>()			
			;
		// .add_systems(Startup, ready);
	}
}



/// Sent from this app, usually once assets are ready.
#[derive(Debug, Clone, Serialize, Deserialize, Event, Reflect)]
pub struct AppReady;
