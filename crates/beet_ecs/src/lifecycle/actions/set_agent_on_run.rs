use crate::prelude::*;
use bevy::prelude::*;

/// Sets an agent's component when this behavior starts running.
/// This does nothing if the agent does not have the component.
#[derive(PartialEq, Deref, DerefMut, Debug, Clone, Component, Action, Reflect)]
#[reflect(Component, ActionMeta)]
#[category(ActionCategory::Agent)]
#[observers(set_agent_on_run::<T>)]
pub struct SetAgentOnRun<T: GenericActionComponent>(pub T);

impl<T: GenericActionComponent> SetAgentOnRun<T> {
	pub fn new(value: impl Into<T>) -> Self { Self(value.into()) }
}

impl<T: Default + GenericActionComponent> Default for SetAgentOnRun<T> {
	fn default() -> Self { Self(T::default()) }
}

fn set_agent_on_run<T: GenericActionComponent>(
	trigger: Trigger<OnRun>,
	mut agents: Query<&mut T>,
	query: Query<(&TargetAgent, &SetAgentOnRun<T>)>,
) {
	let (target, action) = query
		.get(trigger.entity())
		.expect(expect_action::ACTION_QUERY_MISSING);

	if let Ok(mut dst) = agents.get_mut(**target) {
		*dst = action.0.clone();
	} else {
		log::warn!("SetAgentOnRun: Agent with component not found");
	}
}
