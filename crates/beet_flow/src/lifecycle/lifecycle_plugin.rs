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
				InsertOnRun<Running>,
				RemoveOnRunResult<Running>,
				RunOnRunResult,
				RunOnSceneReady,
			)>::default(),
			// repeat flow
			ActionPlugin::<(
				RepeatFlow,
				SucceedTimes,
				InsertOnRun<RepeatFlow>,
				RemoveOnRun<RepeatFlow>,
				InsertOnRunResult<RepeatFlow>,
				RemoveOnRunResult<RepeatFlow>,
			)>::default(),
			// misc
			ActionPlugin::<(
				EmptyAction,
				RunTimer,
				LogOnRun,
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
		// running
		.register_type::<Running>()
		.register_type::<RunResult>()
		// graph
		.register_type::<Parent>()
		.register_type::<Children>()
		.register_type::<BeetRoot>()
		.register_type::<RootIsTargetEntity>()
		.register_type::<TargetEntity>()
		/*-*/;

		// net
		#[cfg(feature = "bevyhub")]
		app.add_plugins(BeetNetPlugin);

		let world = app.world_mut();

		// running
		world.register_component::<Running>();
		// graph
		world.register_component::<Parent>();
		world.register_component::<Children>();
		world.register_component::<BeetRoot>();
		world.register_component::<RootIsTargetEntity>();
		world.register_component::<TargetEntity>();
		// bevy
		world.register_component::<Name>();
		world.register_component::<Transform>();
		world.register_component::<GlobalTransform>();
	}
}
