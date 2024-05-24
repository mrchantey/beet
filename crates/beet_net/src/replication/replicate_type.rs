use crate::prelude::*;
use bevy::ecs::schedule::SystemConfigs;
use bevy::prelude::*;
use std::marker::PhantomData;



/// Base trait for any [`Component`], [`Resource`], or [`Event`] that can be replicated.
pub trait ReplicateType: 'static + Send + Sync {
	fn register(registrations: &mut Registrations);
	fn outgoing_systems() -> SystemConfigs;
}



pub struct ReplicateTypePlugin<T: ReplicateType> {
	phantom: PhantomData<T>,
}

impl<T: ReplicateType> Default for ReplicateTypePlugin<T> {
	fn default() -> Self {
		Self {
			phantom: PhantomData,
		}
	}
}


impl<T: ReplicateType> Plugin for ReplicateTypePlugin<T> {
	fn build(&self, app: &mut App) {
		app.init_resource::<Registrations>();
		let mut registrations = app.world_mut().resource_mut::<Registrations>();
		T::register(&mut registrations);
		app.add_systems(
			Update,
			T::outgoing_systems().in_set(MessageOutgoingSet),
		);
	}
}


// macro_rules! impl_replicate_component_tuples {
// 	($($param: ident),*) => {
// 			impl<$($param),*> ReplicateComponent for ($($param,)*)
// 			where
// 					$($param: Send + Sync + 'static + Component + Serialize + DeserializeOwned),*
// 			{
// 					#[allow(non_snake_case, unused_variables)]
// 					#[track_caller]
// 					fn register(registrations:&mut Registrations) {
// 							$($param::register(registrations);)*
// 					}
// 					fn update_systems()-> SystemConfigs {
// 							($($param::update_systems(),)*).into_configs()
// 					}
// 			}
// 	}
// }
// all_tuples!(impl_replicate_component_tuples, 1, 15, P);
