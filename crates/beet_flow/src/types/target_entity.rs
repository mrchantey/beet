use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::query::QueryData;
use bevy::ecs::query::QueryFilter;

/// General purpose type for specifying the target for an action to perform
/// an operation on, for example [`Insert`] and [`Remove`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the [`ActionTrigger::event_target`]
	#[default]
	Target,
	/// Use the [`AgentQuery::entity`]
	Agent,
	/// Use the [`ChildOf`] for the [`ActionTrigger::event_target`],
	/// defaulting back to [`TargetEntity::Target`] if none present
	Parent,
	/// Specify some other entity to target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for the given trigger.
	pub fn select_target<D, F>(
		&self,
		ev: &On<impl ActionEvent>,
		agents: &GlobalAgentQuery<D, F>,
	) -> Entity
	where
		D: 'static + QueryData,
		F: 'static + QueryFilter,
	{
		match self {
			TargetEntity::Target => ev.event_target(),
			TargetEntity::Agent => ev.agent(),
			TargetEntity::Parent => agents
				.parents
				.get(ev.event_target())
				.ok()
				.and_then(|parent| Some(parent.get()))
				.unwrap_or(ev.event_target()),
			TargetEntity::Other(entity) => *entity,
		}
	}
}
