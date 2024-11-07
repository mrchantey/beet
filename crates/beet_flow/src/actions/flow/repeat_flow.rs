use crate::prelude::*;
use bevy::prelude::*;




/// Reattaches the [`RunOnSpawn`] component whenever [`OnRunResult`] is called.
/// This does **not** trigger observers, which avoids infinite loops.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(repeat)]
pub struct RepeatFlow {
	// TODO times
	// pub times: RepeatAnimation,
}

impl Default for RepeatFlow {
	fn default() -> Self {
		Self {
			// times: RepeatAnimation::Forever,
		}
	}
}

fn repeat(trigger: Trigger<OnRunResult>, mut commands: Commands) {
	commands.entity(trigger.entity()).insert(RunOnSpawn);
}
