use crate::prelude::*;
use bevy::prelude::*;




// /// This does **not** trigger observers, making it safe from infinite loops
// /// Reattaches the [`RunOnSpawn`] component whenever [`OnRunResult`] is called.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(repeat)]
pub struct Repeat {
	// TODO times
	// pub times: RepeatAnimation,
}

impl Default for Repeat {
	fn default() -> Self {
		Self {
			// times: RepeatAnimation::Forever,
		}
	}
}

fn repeat(trigger: Trigger<OnRunResult>, mut commands: Commands) {
	commands.entity(trigger.entity()).insert(RunOnSpawn);
}
