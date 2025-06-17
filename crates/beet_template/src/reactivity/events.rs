use bevy::prelude::*;

pub type OnClick = BeetEvent<MouseEvent>;


#[derive(Debug, Clone, Event, Deref, DerefMut)]
pub struct BeetEvent<T>(pub T);
impl<T> BeetEvent<T> {
	/// Create a new event with the given value
	pub fn new(value: T) -> Self { Self(value) }
}


#[cfg(not(target_arch = "wasm32"))]
pub use event_types_native::*;
#[cfg(not(target_arch = "wasm32"))]
mod event_types_native {
	use super::*;

	pub struct MockEvent {
		pub value: String,
	}
	impl MockEvent {
		pub fn new(value: impl Into<String>) -> Self {
			Self {
				value: value.into(),
			}
		}
	}
	pub type MouseEvent = MockEvent;
	// pub type Event = MockEvent;

	impl EventExt for MockEvent {
		fn value(&self) -> String { self.value.clone() }
	}
}

#[cfg(target_arch = "wasm32")]
pub use event_types_wasm::*;
#[cfg(target_arch = "wasm32")]
mod event_types_wasm {
	use send_wrapper::SendWrapper;
	use wasm_bindgen::JsCast;

	use super::*;
	// pub type Event = web_sys::MouseEvent;
	pub type MouseEvent = SendWrapper<web_sys::MouseEvent>;
	impl EventExt for MouseEvent {
		fn value(&self) -> String {
			self.current_target()
				.unwrap()
				.unchecked_into::<web_sys::HtmlInputElement>()
				.value()
		}
	}
}

pub trait EventExt {
	/// Shorthand for `event.current_target().value()`
	fn value(&self) -> String;
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::as_beet::*;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(String::new());

		App::new()
			.world_mut()
			.spawn(rsx! {<button onclick=move|ev|set(ev.value())/>})
			.trigger(OnClick::new(MockEvent::new("foo")));
		get().xpect().to_be("foo");
	}
}
