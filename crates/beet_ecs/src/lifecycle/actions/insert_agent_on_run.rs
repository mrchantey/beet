use crate::prelude::*;
use bevy::prelude::*;

/// Inserts a component on the agent when this behavior starts running.
#[derive(
	PartialEq, Deref, DerefMut, Debug, Clone, Component, Action, Reflect,
)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[systems(set_agent_on_run::<T>.in_set(PostTickSet))]
pub struct InsertAgentOnRun<T: GenericActionComponent>(pub T);

impl<T: Default + GenericActionComponent> Default for InsertAgentOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionComponent> InsertAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

fn set_agent_on_run<T: GenericActionComponent>(
	mut commands: Commands,
	mut query: Query<(&TargetAgent, &InsertAgentOnRun<T>), Added<Running>>,
) {
	for (agent, src) in query.iter_mut() {
		commands.entity(agent.0).insert(src.0.clone());
	}
}
