use bevy::prelude::*;

pub type OnClick = BeetEvent<MouseEvent>;
pub type OnChange = BeetEvent<BaseEvent>;
pub type OnInput = BeetEvent<BaseEvent>;


#[derive(Debug, Clone, Event, Deref, DerefMut)]
pub struct BeetEvent<T>(pub T);
impl<T> BeetEvent<T> {
	/// Create a new event with the given value
	pub fn new(value: T) -> Self { Self(value) }
}
impl BeetEvent<()> {
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
			"oninput" => {
				let ev = ev.unchecked_into::<web_sys::Event>();
				commands.trigger(BeetEvent::new(SendWrapper::new(ev)));
			}
			"onchange" => {
				let ev = ev.unchecked_into::<web_sys::Event>();
				commands.trigger(BeetEvent::new(SendWrapper::new(ev)));
			}
			other => {
				unimplemented!(
					"typing for html event {other} not yet implemented, please open an issue"
				)
			}
		}
	}
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
	pub type BaseEvent = MockEvent;
	pub type MouseEvent = MockEvent;

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
	pub type BaseEvent = SendWrapper<web_sys::Event>;
	pub type MouseEvent = SendWrapper<web_sys::MouseEvent>;
	macro_rules! impl_event_ext {
		($event_type:ty) => {
			impl EventExt for $event_type {
				fn value(&self) -> String {
					self.current_target()
						.unwrap()
						.unchecked_into::<web_sys::HtmlInputElement>()
						.value()
				}
			}
		};
	}

	impl_event_ext!(MouseEvent);
	impl_event_ext!(BaseEvent);
}

pub trait EventExt {
	/// Shorthand for `event.current_target().value()`
	fn value(&self) -> String;
}


#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod test {
	use crate::as_beet::*;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use sweet::prelude::*;

	#[test]
	fn works() {
		let (get, set) = signal(String::from("foo"));

		let mut app = App::new();
		let world = app.world_mut();
		let entity = world
			.spawn(rsx! {<button onclick=move|ev|set(ev.value())/>})
			.get::<Children>()
			.unwrap()[0];
		world
			.run_system_once(apply_snippets_to_instances)
			.unwrap()
			.unwrap();
		world
			.entity_mut(entity)
			.trigger(OnClick::new(MockEvent::new("bar")));
		get().xpect().to_be("bar");
	}
}
