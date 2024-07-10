use crate::prelude::*;
use bevy::prelude::*;
use std::marker::PhantomData;

/// Removes a component on the agent when this behavior starts running.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Action, Reflect)]
#[reflect(Component)]
#[category(ActionCategory::Agent)]
#[observers(remove_agent_on_run::<T>)]
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
	trigger: Trigger<OnRun>,
	mut commands: Commands,
	query: Query<(&TargetAgent, &RemoveAgentOnRun<T>)>,
) {
	let (agent, _) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);
	commands.entity(agent.0).remove::<T>();
}
