use crate::prelude::*;
use beet_core::prelude::*;

/// General purpose type for specifying the target for an action to perform
/// an operation on, for example [`InsertOn`] and [`RemoveOn`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the event target entity (the action itself)
	#[default]
	Action,
	/// Use the agent entity resolved via [`AgentQuery`]
	Agent,
	/// Specify some other entity to target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for the given action using an [`AgentQuery`] to resolve agent.
	pub fn get(&self, action: Entity, agent_query: &AgentQuery) -> Entity {
		match self {
			TargetEntity::Action => action,
			TargetEntity::Agent => agent_query.entity(action),
			TargetEntity::Other(entity) => *entity,
		}
	}
}
