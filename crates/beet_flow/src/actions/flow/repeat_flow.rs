use crate::prelude::*;
use bevy::prelude::*;




/// Reattaches the [`RunOnSpawn`] component whenever [`OnRunResult`] is called.
/// Using [`RunOnSpawn`] means this does **not** directly trigger observers, which avoids infinite loops.
///
/// Note that [RepeatFlow] requires [NoBubble] so results must be bubbled up manually.
#[derive(Debug, Clone, PartialEq, Component, Action, Reflect)]
#[reflect(Default, Component, ActionMeta)]
#[category(ActionCategory::Behavior)]
#[observers(repeat)]
#[require(NoBubble)]
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

fn repeat(
	trigger: Trigger<OnRunResult>,
	// names: Query<&Name>,
	mut commands: Commands,
) {
	// println!("repeat for {}", name_or_entity(&names, trigger.entity()));
	commands.entity(trigger.entity()).insert(RunOnSpawn);
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;


	// use sweet::prelude::*;

	#[test]
	fn removes_running() {
		let mut app = App::new();
		app.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<(SequenceFlow, RepeatFlow)>::default(),
		));
		let world = app.world_mut();
		let _root = world
			.spawn((Name::new("root"), Running, RepeatFlow::default()))
			.with_child(Running);

		// expect(true).to_be_false();
	}
}
