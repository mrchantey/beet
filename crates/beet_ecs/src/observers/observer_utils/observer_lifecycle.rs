use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;
use bevy::utils::all_tuples;

pub struct ObserverLifecycleTupleMarker;
pub struct ObserverLifecycleSystemMarker;

pub trait ObserverLifecycle<M>: Sized {
	fn spawn_observers(
		&self,
		commands: &mut Commands,
		entity: Entity,
	) -> Vec<Entity>;
}

impl ObserverLifecycle<()> for () {
	fn spawn_observers(&self, _: &mut Commands, _: Entity) -> Vec<Entity> {
		Vec::new()
	}
}


impl<E: Event, B: Bundle, M, T>
	ObserverLifecycle<(E, B, M, ObserverLifecycleSystemMarker)> for T
where
	T: IntoObserverSystem<E, B, M> + Clone,
{
	fn spawn_observers(
		&self,
		commands: &mut Commands,
		entity: Entity,
	) -> Vec<Entity> {
		vec![commands
			.spawn(Observer::new(self.clone()).with_entity(entity))
			.id()]
	}
}

macro_rules! impl_plugins_tuples {
	($(($param: ident,$marker:ident)),*) => {
			impl<$($param, $marker),*> ObserverLifecycle<(($($marker,)*),ObserverLifecycleTupleMarker)> for ($($param,)*)
			where
					$($param: ObserverLifecycle<$marker>),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn spawn_observers(
							&self,
							commands: &mut Commands,
							entity: Entity,
					) -> Vec<Entity> {
							let ($($param,)*) = self;
							let mut entities = Vec::new();
							$(
									entities.extend($param.spawn_observers(commands, entity));
							)*
							entities
					}
			}
	}
}
all_tuples!(impl_plugins_tuples, 1, 15, P, M);
