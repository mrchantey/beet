use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Inserts a component on the agent when this behavior starts running.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct InsertAgentOnRun<T: GenericActionComponent>(pub T);

impl<T: Default + GenericActionComponent> Default for InsertAgentOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

impl<T: GenericActionComponent> InsertAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}


impl<T: GenericActionComponent> ActionMeta for InsertAgentOnRun<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl<T: GenericActionComponent> ActionSystems for InsertAgentOnRun<T> {
	fn systems() -> SystemConfigs { set_agent_on_run::<T>.in_set(PostTickSet) }
}


fn set_agent_on_run<T: GenericActionComponent>(
	mut commands: Commands,
	mut query: Query<(&TargetAgent, &InsertAgentOnRun<T>), Added<Running>>,
) {
	for (agent, src) in query.iter_mut() {
		commands.entity(agent.0).insert(src.0.clone());
	}
}
