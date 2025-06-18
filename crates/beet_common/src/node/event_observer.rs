use beet_common_macros::ImplBundle;
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
		entity.insert(self.observer);
	}
}

#[derive(Default, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Default, Component)]
pub struct EventKey {
	/// The unchanged event name used in the template, which
	/// may be one of several casings, ie
	/// `onmousemove`, `onMouseMove`, `OnMouseMove`
	name: String,
}

impl EventKey {
	/// Create a new event observer with the given name
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
		}
	}
	/// Get the event name in a consistent lowercase format
	pub fn event_name(&self) -> String { self.name.to_lowercase() }
}
