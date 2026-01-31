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
/// # use beet_flow::prelude::*;
/// # let mut world = World::new();
/// // Insert Running on the agent when GetOutcome is triggered
/// world.spawn(InsertOn::<GetOutcome, Running>::new_with_target(
///     Running,
///     TargetEntity::Agent,
/// ));
/// ```
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the event target entity (the action itself).
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
}
