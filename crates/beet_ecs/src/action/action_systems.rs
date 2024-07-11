use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::all_tuples;

pub trait ActionSystems: Sized {
	fn on_build(app: &mut App, config: &BeetConfig);
}


macro_rules! impl_plugins_tuples {
	($($param: ident),*) => {
			impl<$($param),*> ActionSystems for ($($param,)*)
			where
					$($param: ActionSystems),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn on_build(app:&mut App, config: &BeetConfig){
						$(
								$param::on_build(app, config);
						)*
							// ($($param::systems(),)*).into_configs()
					}
			}
	}
}
all_tuples!(impl_plugins_tuples, 1, 15, P);
