use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use bevy::utils::all_tuples;

pub trait ActionSystems: Sized {
	fn systems() -> SystemConfigs;
}


macro_rules! impl_plugins_tuples {
	($($param: ident),*) => {
			impl<$($param),*> ActionSystems for ($($param,)*)
			where
					$($param: ActionSystems),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn systems()-> SystemConfigs {
							($($param::systems(),)*).into_configs()
					}
			}
	}
}
all_tuples!(impl_plugins_tuples, 1, 15, P);
