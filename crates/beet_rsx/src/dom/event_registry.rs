use crate::prelude::*;

pub trait EventHandler<T>: 'static + Send + Sync + Fn(T) {}
impl<T, F> EventHandler<T> for F where F: 'static + Send + Sync + Fn(T) {}



#[cfg(not(target_arch = "wasm32"))]
pub use event_types_native::*;
#[cfg(not(target_arch = "wasm32"))]
mod event_types_native {
	pub struct MockEvent {
		pub target: MockTarget,
	}
	pub struct MockTarget {
		pub value: String,
	}
	pub type MouseEvent = MockEvent;
	pub type Event = MockEvent;
}

#[cfg(target_arch = "wasm32")]
pub use event_types_wasm::*;
#[cfg(target_arch = "wasm32")]
mod event_types_wasm {
	// use super::*;
	pub type Event = web_sys::MouseEvent;
	pub type MouseEvent = web_sys::MouseEvent;
	// impl EventExt for MouseEvent {
	// 	fn value(&self) -> String {
	// 		self.current_target()
	// 			.into::<web_sys::HtmlInputElement>()
	// 			.unwrap()
	// 			.value()
	// 	}
	// }
}



pub struct EventRegistry;



impl EventRegistry {
	pub fn initialize() -> ParseResult<()> {
		#[cfg(target_arch = "wasm32")]
		DomEventRegistry::initialize()?;
		Ok(())
	}

	#[cfg(target_arch = "wasm32")]
	pub fn register<T: 'static + wasm_bindgen::JsCast>(
		key: &str,
		loc: TreeLocation,
		func: impl EventHandler<T>,
	) {
		DomEventRegistry::register(key, loc, func);
	}
	#[cfg(not(target_arch = "wasm32"))]
	pub fn register<T: 'static>(
		_key: &str,
		_loc: TreeLocation,
		_func: impl EventHandler<T>,
	) {
		todo!("rsdom should handle this");
	}

	/// Typed handler for `onclick` events.
	pub fn register_onclick(
		key: &str,
		loc: TreeLocation,
		value: impl EventHandler<MouseEvent>,
	) {
		Self::register(key, loc, value);
	}
	/// Typed handler for `onclick` events.
	pub fn register_onchange(
		key: &str,
		loc: TreeLocation,
		value: impl EventHandler<Event>,
	) {
		Self::register(key, loc, value);
	}
}


pub trait EventExt {
	/// Shorthand for `event.current_target().value()`
	fn value(&self) -> String;
}



#[cfg(test)]
mod test {
	// use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	#[cfg(target_arch = "wasm32")]
	fn works() {}
}
