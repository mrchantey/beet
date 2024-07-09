use crate::prelude::*;
use bevy::prelude::*;


/// Provides the core actions and utility systems to manage behavior lifecycles.
#[derive(Default)]
pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(LifecycleSystemsPlugin);

		app.add_plugins(ActionPlugin::<(
			InsertInDuration<RunResult>,
			InsertOnRun<RunResult>,
			LogOnRun,
			// CallOnRun,
			Repeat,
			SetOnSpawn<Score>,
			// selectors
			FallbackSelector,
			ParallelSelector,
			SequenceSelector,
			ScoreSelector,
			// utility
			EmptyAction,
		)>::default())
		.add_plugins(ActionPlugin::<(
			// ContinueRun,
			InsertOnTrigger<OnRun, Running>,
			RemoveOnTrigger<OnRunResult, Running>,
			SequenceFlow,
			EndOnRun,
			RunOnSpawn,
		)>::default())
		// observers
		.register_type::<ContinueRun>()
		.register_type::<RunOnSpawn>()
		// running
		.register_type::<Running>()
		.register_type::<RunTimer>()
		.register_type::<RunResult>()
		// graph
		.register_type::<Parent>()
		.register_type::<Children>()
		.register_type::<BeetRoot>()
		.register_type::<RootIsTargetAgent>()
		.register_type::<TargetAgent>()
		.register_type::<ActionTarget>()
		/*-*/;

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
