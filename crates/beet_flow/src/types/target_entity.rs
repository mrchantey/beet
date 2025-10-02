use crate::types::AgentQuery;
use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;

/// General purpose type for specifying the target for an action to perform
/// an operation on, for example [`Insert`] and [`Remove`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the [`EntityEvent::event_target`]
	#[default]
	Target,
	/// Use the [`AgentQuery::entity`]
	Agent,
	/// Specify some other entity to target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for the given trigger.
	pub fn select_target<E: EntityEvent, D, F>(
		&self,
		ev: &On<E>,
		agents: &AgentQuery<D, F>,
	) -> Entity
	where
		D: 'static + QueryData,
		F: 'static + QueryFilter,
	{
		match self {
			TargetEntity::Target => ev.event_target(),
			TargetEntity::Agent => agents.entity(ev.event_target()),
			TargetEntity::Other(entity) => *entity,
		}
	}
}
