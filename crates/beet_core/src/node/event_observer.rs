use beet_core_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::ecs::system::IntoObserverSystem;
use bevy::prelude::*;

/// A typed version of an [`EntityObserver`](beet_bevy::prelude::EntityObserver) to help with type inference
#[derive(ImplBundle)]
pub struct EventHandler<E: 'static + Send + Sync + Event> {
	observer: Observer,
	_phantom: std::marker::PhantomData<E>,
}

impl<E: 'static + Send + Sync + Event> EventHandler<E> {
	/// Create a new event handler for the given event type
	pub fn new<B: Bundle, M>(
		handler: impl IntoObserverSystem<E, B, M>,
	) -> Self {
		Self {
			observer: Observer::new(handler),
			_phantom: std::marker::PhantomData,
		}
	}
}

impl<E: 'static + Send + Sync + Event> BundleEffect for EventHandler<E> {
	fn apply(self, entity: &mut EntityWorldMut) {
		entity.insert((EventTarget, self.observer.with_entity(entity.id())));
	}
}


/// Marker type added to any entity with an event handler, often
/// as an attribute.
#[derive(Default, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Default, Component)]
pub struct EventTarget;
