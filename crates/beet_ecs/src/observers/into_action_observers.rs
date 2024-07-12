use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;
use bevy::utils::all_tuples;

pub struct IntoActionObserversObserver;
pub struct IntoActionObserversTupleMarker;
pub struct IntoActionObserversSystemMarker;

pub trait IntoActionObservers<M>: 'static + Send + Sync + Sized {
	fn spawn_observers(
		self,
		commands: &mut Commands,
		entity: Entity,
	) -> Vec<Entity>;
}

impl IntoActionObservers<()> for () {
	fn spawn_observers(self, _: &mut Commands, _: Entity) -> Vec<Entity> {
		Vec::new()
	}
}


impl<E: Event, B: Bundle, M>
	IntoActionObservers<(E, B, M, IntoActionObserversObserver)> for Observer<E, B>
{
	fn spawn_observers(
		self,
		commands: &mut Commands,
		_entity: Entity,
	) -> Vec<Entity> {
		vec![commands.spawn(self).id()]
	}
}
impl<E: Event, B: Bundle, M, T: 'static + Send + Sync>
	IntoActionObservers<(E, B, M, IntoActionObserversSystemMarker)> for T
where
	T: IntoObserverSystem<E, B, M> + Clone,
{
	fn spawn_observers(
		self,
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
			impl<$($param, $marker),*> IntoActionObservers<(($($marker,)*),IntoActionObserversTupleMarker)> for ($($param,)*)
			where
					$($param: IntoActionObservers<$marker>),*
			{
					#[allow(non_snake_case, unused_variables)]
					#[track_caller]
					fn spawn_observers(
							self,
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
