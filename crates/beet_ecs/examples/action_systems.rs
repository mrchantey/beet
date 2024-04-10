use beet_ecs::prelude::ActionSystems;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::App;
use bevy::utils::all_tuples;


fn main() {}


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
all_tuples!(impl_plugins_tuples, 1, 3, P);
// all_tuples!(impl_plugins_tuples, 1, 15, P, p);
