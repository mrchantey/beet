//! Target entity specification for action operations.
use crate::prelude::*;
use beet_core::prelude::*;

/// Specifies which entity an action should operate on.
///
/// Many actions need a target entity for their operations, such as inserting
/// or removing components. This enum provides a flexible way to specify that
/// target relative to the action entity.
///
/// # Example
///
/// ```
/// # use beet_core::prelude::*;
/// # use beet_action::prelude::*;
/// // Operate on the agent rather than the action entity itself
/// let target = TargetEntity::Agent;
/// # let _ = target;
/// ```
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the action entity itself (the caller).
	#[default]
	Action,
	/// Use the agent entity resolved via [`AgentQuery`].
	Agent,
	/// Specify an explicit entity to target.
	Other(Entity),
}

impl TargetEntity {
	/// Resolves the target entity for the given action.
	///
	/// Uses the provided [`AgentQuery`] to resolve the agent when
	/// [`TargetEntity::Agent`] is specified.
	pub fn get(&self, action: Entity, agent_query: &AgentQuery) -> Entity {
		match self {
			TargetEntity::Action => action,
			TargetEntity::Agent => agent_query.entity(action),
			TargetEntity::Other(entity) => *entity,
		}
	}

	/// Resolves the target entity asynchronously, using [`AgentQuery`] to
	/// resolve the agent when [`TargetEntity::Agent`] is specified.
	pub async fn get_async(
		&self,
		world: &AsyncWorld,
		action: Entity,
	) -> Entity {
		match self {
			TargetEntity::Action => action,
			TargetEntity::Agent => {
				AgentQuery::entity_async(world, action).await
			}
			TargetEntity::Other(entity) => *entity,
		}
	}
}
