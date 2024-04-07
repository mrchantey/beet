use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;

#[reflect_trait]
pub trait ActionChildComponents {
	fn insert_child_components(&self, entity: &mut EntityWorldMut<'_>);
	fn boxed_child_components(&self) -> Vec<Box<dyn Reflect>>;
}

/// A trait for registering systems associated with an action
// - we use this instead of implementing IntoSystemConfigs so that
//   `Default` is not required for the action
// - must be static for use in beet plugin
pub trait ActionSystems: 'static {
	fn add_systems(app: &mut App, schedule: impl ScheduleLabel + Clone);
}