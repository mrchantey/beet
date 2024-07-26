use crate::prelude::*;
use beetmash::prelude::*;
use bevy::prelude::*;


pub struct BeetNetPlugin;


pub type RunOnAppReady = TriggerOnGlobalTrigger<AppReady, OnRun>;


impl Plugin for BeetNetPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			ActionPlugin::<(TriggerOnRun<AppReady>, RunOnAppReady)>::default(),
		)
		.observe(log_on_user_message);

		app.configure_sets(
			Update,
			(
				beetmash::prelude::MessageIncomingSet.in_set(PreTickSet),
				beetmash::prelude::MessageOutgoingSet.in_set(PostTickSet),
			),
		);
	}
}


fn log_on_user_message(
	trigger: Trigger<OnUserMessage>,
	mut commands: Commands,
) {
	commands.trigger(OnLogMessage::new(format!("User: {}", &trigger.event().0)))
}
