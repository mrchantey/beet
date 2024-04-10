use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::utils::all_tuples;

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


macro_rules! impl_plugins_tuples {
	($($param: ident),*) => {
			impl<$($param),*> ActionSystems for ($($param,)*)
			where
					$($param: ActionSystems),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn add_systems(app: &mut App,schedule:impl ScheduleLabel + Clone) {
							$($param::add_systems(app, schedule.clone());)*
					}
			}
	}
}
all_tuples!(impl_plugins_tuples, 1, 15, P);
