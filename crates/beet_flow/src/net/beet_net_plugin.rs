use crate::prelude::*;
use bevyhub::prelude::*;
use bevy::prelude::*;


pub struct BeetNetPlugin;


pub type RunOnAppReady = TriggerOnGlobalTrigger<OnRun, AppReady>;


impl Plugin for BeetNetPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			ActionPlugin::<(TriggerOnRun<AppReady>, RunOnAppReady)>::default(),
		)
		.configure_sets(
			Update,
			(
				bevyhub::prelude::MessageIncomingSet.in_set(PreTickSet),
				bevyhub::prelude::MessageOutgoingSet.in_set(PostTickSet),
			),
		);
	}
}
