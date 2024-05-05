use crate::prelude::*;
use bevy::prelude::*;


#[derive(Default)]
pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(LifecycleSystemsPlugin);

		app.add_plugins(ActionPlugin::<(
			InsertInDuration<RunResult>,
			InsertOnRun<RunResult>,
			LogNameOnRun,
			LogOnRun,
			Repeat,
			SetOnSpawn<Score>,
			// selectors
			FallbackSelector,
			ParallelSelector,
			SequenceSelector,
			ScoreSelector,
			// utility
			EmptyAction,
		)>::default());


		let world = app.world_mut();

		// running
		world.init_component::<Running>();
		world.init_component::<RunTimer>();
		world.init_component::<RunResult>();
		// graph
		world.init_component::<Parent>();
		world.init_component::<Children>();
		world.init_component::<BeetRoot>();
		world.init_component::<RootIsTargetAgent>();
		world.init_component::<TargetAgent>();
		world.init_component::<ActionTarget>();
		// bevy
		world.init_component::<Name>();
		world.init_component::<Transform>();
		world.init_component::<GlobalTransform>();
	}
}
