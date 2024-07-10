use crate::prelude::*;
use beet_ecs::prelude::*;
use bevy::prelude::*;
use serde::Deserialize;
use serde::Serialize;


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
		app
			// AppStartup
			.add_event::<AppStartup>()
			.replicate_event_outgoing::<AppStartup>()
			.add_systems(Startup, |mut events: EventWriter<AppStartup>| {
				events.send(AppStartup);
			})
			// AppReady
			.add_event::<AppReady>()
			.replicate_event_outgoing::<AppReady>()
			.add_plugins(ActionPlugin::<(
				SendOnRun<AppReady>,
				TriggerOnRun<AppReady>,
				RunOnAppReady,
			)>::default())
			// SpawnSceneFile
			.add_event::<SpawnSceneFile>()
			.replicate_event_incoming::<SpawnSceneFile>()
			.add_event::<SpawnSceneFileResponse>()
			.replicate_event_outgoing::<SpawnSceneFileResponse>()
			.add_systems(Update, handle_spawn_scene);
		// Screenshot

		#[cfg(not(test))]
		app.add_event::<SaveScreenshot>()
		.replicate_event_incoming::<SaveScreenshot>()
		.add_systems(Update,screenshot_on_event)
		.add_systems(Update,screenshot_on_keypress)
		// .observe(screenshot_on_event)
		// .observe(screenshot_on_keypress)
			/*-*/;
	}
}

/// Sent from this app on the Startup schedule.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Event, Reflect)]
#[reflect(Default)]
pub struct AppStartup;
/// Sent from this app, usually once assets are ready.
#[derive(Debug, Default, Clone, Serialize, Deserialize, Event, Reflect)]
#[reflect(Default)]
pub struct AppReady;

pub type RunOnAppReady = TriggerOnGlobalTrigger<AppReady, OnRun>;
