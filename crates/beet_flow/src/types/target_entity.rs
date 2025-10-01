use beet_core::prelude::*;

/// General purpose type for specifying the target for an action to perform
/// an operation on, for example [`Insert`] and [`Remove`].
#[derive(Debug, Default, Clone, Component, Reflect, PartialEq, Eq, Hash)]
#[reflect(Default, Component)]
pub enum TargetEntity {
	/// Use the `trigger::event_target`
	#[default]
	Target,
	/// Use the `trigger::original_event_target`
	OriginalTarget,
	/// Specify some other entity to target
	Other(Entity),
}

impl TargetEntity {
	/// Get the target entity for the given trigger.
	pub fn get_target<E: EntityTargetEvent>(&self, ev: &On<E>) -> Entity {
		match self {
			TargetEntity::Target => E::event_target(ev.trigger()),
			TargetEntity::OriginalTarget => {
				E::original_event_target(ev.trigger())
			}
			TargetEntity::Other(entity) => *entity,
		}
	}
}
