use crate::prelude::*;
use beet_core::prelude::*;

/// General purpose type for specifying the target for an action to perform
/// an operation on, for example [`Insert`] and [`Remove`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the [`ActionContext::event_target`]
	#[default]
	Action,
	/// Use the [`ActionContext::agent`]
	Agent,
	/// Specify some other entity to target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for the given trigger.
	pub fn select_target(&self, ev: &On<impl ActionEvent>) -> Entity {
		match self {
			TargetEntity::Action => ev.event_target(),
			TargetEntity::Agent => ev.agent(),
			TargetEntity::Other(entity) => *entity,
		}
	}
}
