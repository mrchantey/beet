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
			.add_systems(Update, handle_spawn_scene)
			.replicate_observer_incoming::<OnUserMessage>()
			.observe(log_on_user_message)
			.replicate_observer_outgoing::<OnAppMessage>();
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




/// User messages received either internally or externally, can be treated like an StdIn.
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct OnUserMessage(pub String);
/// App messages for outputting, can be treated like an StdOut.
#[derive(
	Debug, Clone, Deref, DerefMut, Serialize, Deserialize, Event, Reflect,
)]
pub struct OnAppMessage(pub String);


fn log_on_user_message(
	trigger: Trigger<OnUserMessage>,
	mut commands: Commands,
) {
	commands.trigger(OnLogMessage::new(format!("User: {}", &trigger.event().0)))
}
