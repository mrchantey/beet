use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Removes a component on the agent when this behavior starts running.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Action, Reflect)]
#[reflect(Component)]
#[category(ActionCategory::Agent)]
#[systems(remove_agent_on_run::<T>.in_set(PostTickSet))]
pub struct RemoveAgentOnRun<T: GenericActionComponent>(
	#[reflect(ignore)] pub PhantomData<T>,
);

impl<T: GenericActionComponent> Default for RemoveAgentOnRun<T> {
	fn default() -> Self { Self(PhantomData) }
}

// impl<T: GenericActionComponent> RemoveAgentOnRun<T> {
// 	pub fn new() -> Self { Self::default() }
// }

fn remove_agent_on_run<T: GenericActionComponent>(
	mut commands: Commands,
	mut query: Query<(&TargetAgent, &RemoveAgentOnRun<T>), Added<Running>>,
) {
	for (agent, _) in query.iter_mut() {
		commands.entity(agent.0).remove::<T>();
	}
}
