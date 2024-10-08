use crate::prelude::*;
use bevy::prelude::*;
use bevy::utils::all_tuples;


/// Trait implemented for actions, allowing them to register their
/// components, systems and observers.
pub trait ActionBuilder: Sized {
	fn build(app: &mut App, config: &BeetConfig);
}


macro_rules! impl_plugins_tuples {
	($($param: ident),*) => {
			impl<$($param),*> ActionBuilder for ($($param,)*)
			where
					$($param: ActionBuilder),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn build(app:&mut App, config: &BeetConfig){
						$(
								$param::build(app, config);
						)*
							// ($($param::systems(),)*).into_configs()
					}
			}
	}
}
all_tuples!(impl_plugins_tuples, 1, 15, P);
