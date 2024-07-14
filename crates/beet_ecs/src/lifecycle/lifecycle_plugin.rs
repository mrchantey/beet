use crate::prelude::*;
use bevy::prelude::*;


/// Provides the core actions and utility systems to manage behavior lifecycles.
#[derive(Default)]
pub struct LifecyclePlugin;

impl Plugin for LifecyclePlugin {
	fn build(&self, app: &mut App) {
		app
		.add_plugins((
			LifecycleSystemsPlugin,
			ActionPlugin::<(
				EndOnRun,
				TriggerInDuration<OnRunResult>,
				InsertOnTrigger<OnRun, Running>,
				RemoveOnTrigger<OnRunResult, Running>,
			)>::default(),
			ActionPlugin::<(
			EmptyAction,
			RunTimer,
			LogOnRun,
			Repeat,
			SequenceFlow,
			FallbackFlow,
			ParallelFlow,
			ScoreFlow,
			ScoreProvider,
			RunOnSpawn,
		)>::default()
		))
		// observers
		.register_type::<ContinueRun>()
		.register_type::<RunOnSpawn>()
		// running
		.register_type::<Running>()
		.register_type::<RunResult>()
		// graph
		.register_type::<Parent>()
		.register_type::<Children>()
		.register_type::<BeetRoot>()
		.register_type::<RootIsTargetAgent>()
		.register_type::<TargetAgent>()
		/*-*/;

		let world = app.world_mut();

		// running
		world.init_component::<Running>();
		// graph
		world.init_component::<Parent>();
		world.init_component::<Children>();
		world.init_component::<BeetRoot>();
		world.init_component::<RootIsTargetAgent>();
		world.init_component::<TargetAgent>();
		// bevy
		world.init_component::<Name>();
		world.init_component::<Transform>();
		world.init_component::<GlobalTransform>();
	}
}
