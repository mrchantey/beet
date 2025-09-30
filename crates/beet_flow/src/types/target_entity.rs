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
	pub fn get_target<
		'a,
		const AUTO_PROPAGATE: bool,
		E: Event<Trigger<'a> = EntityTargetTrigger<AUTO_PROPAGATE, E, T>>,
		T: 'static + Send + Sync + Traversal<E>,
	>(
		&self,
		trigger: &EntityTargetTrigger<AUTO_PROPAGATE, E, T>,
	) -> Entity {
		match self {
			TargetEntity::Target => trigger.event_target(),
			TargetEntity::OriginalTarget => trigger.original_event_target(),
			TargetEntity::Other(entity) => *entity,
		}
	}
}
