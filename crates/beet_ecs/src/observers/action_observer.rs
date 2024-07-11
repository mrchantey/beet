use crate::prelude::*;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;

pub struct ActionObserver<T: ObserverType> {
	pub trigger_target: TriggerTarget,
	pub observer: T::System,
}

impl<T: ObserverType> ActionObserver<T> {
	pub fn new(observer: T::System) -> Self {
		Self {
			observer,
			trigger_target: TriggerTarget::default(),
		}
	}
	pub fn observe(
		&self,
		commands: &mut Commands,
		caller: Entity,
	) -> Vec<Entity> {
		match &self.trigger_target {
			TriggerTarget::This => {
				vec![commands
					.entity(caller)
					.observe(self.observer.clone())
					.id()]
			}
			TriggerTarget::Entity(entity) => {
				vec![commands
					.entity(*entity)
					.observe(self.observer.clone())
					.id()]
			}
			TriggerTarget::Entities(entities) => entities
				.iter()
				.map(|entity| {
					commands.entity(*entity).observe(self.observer.clone()).id()
				})
				.collect(),
			TriggerTarget::Global => {
				vec![commands.observe(self.observer.clone()).id()]
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
		&self,
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
