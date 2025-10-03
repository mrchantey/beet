use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::system::IntoObserverSystem;

/// A typed version of an [`EntityObserver`](beet_bevy::prelude::EntityObserver)
/// which will also insert an [`EventTarget`]
#[derive(BundleEffect)]
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
	fn effect(self, entity: &mut EntityWorldMut) {
		entity.insert(EventTarget);
		let observer = self.observer.with_entity(entity.id());
		entity.world_scope(move |world| {
			world.spawn(observer);
		});
	}
}


/// Marker type added to any entity with an event handler, often
/// as an attribute.
#[derive(Default, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Default, Component)]
#[require(RequiresDomIdx)]
pub struct EventTarget;
