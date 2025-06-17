use beet_bevy::prelude::EntityObserver;
use bevy::ecs::system::ObserverSystem;
use bevy::prelude::*;

#[derive(Bundle)]
pub struct EventBundle<E: 'static + Send + Sync> {
	event_observer: EventObserver,
	// entity_observer: EntityObserver,
	#[bundle(ignore)]
	_phantom: std::marker::PhantomData<E>,
}


pub trait IntoEventObserver<M> {
	/// Used for implicit closure types
	type Event;
	fn into_event_observer(self) -> EntityObserver;
}


impl<E, B, M, S> IntoEventObserver<(S, E, B, M)> for S
where
	S: IntoSystem<Trigger<'static, E, B>, (), M> + Send + 'static,
	S::System: ObserverSystem<E, B, ()>,
	E: 'static + Send + Sync + Event,
	B: Bundle,
{
	type Event = E;
	fn into_event_observer(self) -> EntityObserver { EntityObserver::new(self) }
}


#[derive(Default, Clone, PartialEq, Eq, Hash, Component, Reflect)]
#[reflect(Default, Component)]
pub struct EventObserver {
	/// The unchanged event name used in the template, which
	/// may be one of several casings, ie
	/// `onmousemove`, `onMouseMove`, `OnMouseMove`
	name: String,
}

impl EventObserver {
	/// Create a new event observer with the given name
	pub fn new(name: &str) -> Self {
		Self {
			name: name.to_string(),
		}
	}

	/// Get the event name in a consistent lowercase format
	pub fn event_name(&self) -> String { self.name.to_lowercase() }



	#[cfg(target_arch = "wasm32")]
	pub fn trigger(
		commands: &mut EntityCommands,
		event_name: &str,
		ev: web_sys::Event,
	) {
		use send_wrapper::SendWrapper;
		use wasm_bindgen::JsCast;
		match event_name {
			"onclick" => {
				let ev = ev.unchecked_into::<web_sys::MouseEvent>();
				commands.trigger(BeetEvent::new(SendWrapper::new(ev)));
			}
			_ => unimplemented!(),
		}
	}
}


#[cfg(test)]
mod test {
	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Event)]
	struct Foo;

	#[test]
	fn works() {
		// let foo: Box<dyn IntoEventObserver<_, Event = Foo>> = Box::new(|| {});
	}
}
