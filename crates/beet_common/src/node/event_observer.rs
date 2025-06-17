use beet_common_macros::ImplBundle;
use bevy::ecs::bundle::BundleEffect;
use bevy::prelude::*;
use std::borrow::Cow;


pub trait EventMeta {
	type Payload: 'static + Send + Sync + Event;
	fn name() -> Cow<'static, str>;
}

#[derive(ImplBundle)]
pub struct EventBundle<E: 'static + Send + Sync + EventMeta> {
	event_observer: EventKey,
	observer: Observer,
	_phantom: std::marker::PhantomData<E>,
}

impl<E: 'static + Send + Sync + EventMeta> BundleEffect for EventBundle<E> {
	fn apply(self, entity: &mut EntityWorldMut) {
		entity.insert(self.event_observer);
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
	use std::borrow::Cow;

	use crate::prelude::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[derive(Event)]
	struct Foo;

	impl EventMeta for Foo {
		type Payload = Self;
		fn name() -> Cow<'static, str> { "foo".into() }
	}

	#[test]
	fn works() {
		// let foo: Box<dyn IntoEventObserver<_, Event = Foo>> = Box::new(|| {});
	}
}
