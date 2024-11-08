use crate::prelude::*;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;

pub struct ActionObserver<T: ObserverType> {
	pub trigger_target: ActionTarget,
	pub observer: T::System,
}

impl<T: ObserverType> ActionObserver<T> {
	pub fn new(observer: T::System) -> Self {
		Self {
			observer,
			trigger_target: ActionTarget::default(),
		}
	}
	pub fn observe(
		self,
		commands: &mut Commands,
		caller: Entity,
	) -> Vec<Entity> {
		match &self.trigger_target {
			ActionTarget::This => {
				vec![commands
					.entity(caller)
					.observe(self.observer.clone())
					.id()]
			}
			ActionTarget::Entity(entity) => {
				vec![commands
					.entity(*entity)
					.observe(self.observer.clone())
					.id()]
			}
			ActionTarget::Entities(entities) => entities
				.iter()
				.map(|entity| {
					commands.entity(*entity).observe(self.observer.clone()).id()
				})
				.collect(),
			ActionTarget::Global => {
				vec![commands.add_observer(self.observer.clone()).id()]
			}
		}
	}
}


impl<T: ObserverType>
	IntoActionObservers<(
		T::Event,
		T::Bundle,
		T::Marker,
		IntoActionObserversSystemMarker,
	)> for ActionObserver<T>
{
	fn spawn_observers(
		self,
		commands: &mut Commands,
		entity: Entity,
	) -> Vec<Entity> {
		self.observe(commands, entity)
	}
}

pub trait ObserverType: 'static + Send + Sync {
	type Event: Event;
	type Bundle: Bundle;
	type Marker;
	type System: 'static
		+ Send
		+ Sync
		+ Clone
		+ IntoObserverSystem<Self::Event, Self::Bundle, Self::Marker>;
}
