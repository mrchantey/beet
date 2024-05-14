use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;

/// Sets an agent's component when this behavior starts running.
/// This does nothing if the agent does not have the component.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Component, Reflect)]
#[reflect(Component, ActionMeta)]
pub struct SetAgentOnRun<T: GenericActionComponent>(pub T);

impl<T: GenericActionComponent> SetAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

impl<T: Default + GenericActionComponent> Default for SetAgentOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}


impl<T: GenericActionComponent> ActionMeta for SetAgentOnRun<T> {
	fn category(&self) -> ActionCategory { ActionCategory::Agent }
}

impl<T: GenericActionComponent> ActionSystems for SetAgentOnRun<T> {
	fn systems() -> SystemConfigs { set_agent_on_run::<T>.in_set(PostTickSet) }
}


fn set_agent_on_run<T: GenericActionComponent>(
	mut agents: Query<&mut T>,
	mut query: Query<(&TargetAgent, &SetAgentOnRun<T>), Added<Running>>,
) {
	for (entity, src) in query.iter_mut() {
		if let Ok(mut dst) = agents.get_mut(**entity) {
			*dst = src.0.clone();
		}
	}
}
