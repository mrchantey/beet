use crate::prelude::*;


pub trait EventHandler<T>: 'static + Send + Sync + Fn(T) {
	// fn box_clone(&self) -> Box<dyn EventHandler<T>>;
	// fn call(&self, val: T);
}
impl<T, F> EventHandler<T> for F
where
	F: 'static + Send + Sync + Fn(T),
{
	// fn box_clone(&self) -> Box<dyn EventHandler<T>> { Box::new(self.clone()) }
	// fn call(&self, val: T) { self(val); }
}


#[cfg(not(target_arch = "wasm32"))]
pub use event_types_native::*;
#[cfg(not(target_arch = "wasm32"))]
mod event_types_native {
	use super::*;

	pub struct MockEvent {
		pub target: MockTarget,
	}
	pub struct MockTarget {
		pub value: String,
	}
	pub type MouseEvent = MockEvent;
	pub type Event = MockEvent;

	impl EventExt for MockEvent {
		fn value(&self) -> String { self.target.value.clone() }
	}
}

#[cfg(target_arch = "wasm32")]
pub use event_types_wasm::*;
#[cfg(target_arch = "wasm32")]
mod event_types_wasm {
	use wasm_bindgen::JsCast;

	use super::*;
	pub type Event = web_sys::MouseEvent;
	pub type MouseEvent = web_sys::MouseEvent;
	impl EventExt for MouseEvent {
		fn value(&self) -> String {
			self.current_target()
				.unwrap()
				.unchecked_into::<web_sys::HtmlInputElement>()
				.value()
		}
	}
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
	/// Typed handler for `onchange` events.
	pub fn register_onchange(
		key: &str,
		loc: TreeLocation,
		value: impl EventHandler<Event>,
	) {
		Self::register(key, loc, value);
	}
	/// Typed handler for `oninput` events.
	pub fn register_oninput(
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
	use crate::prelude::*;
	// use sweet::prelude::*;

	#[test]
	#[cfg_attr(not(target_arch = "wasm32"), should_panic)]
	fn works() {
		let func: Box<dyn EventHandler<_>> = Box::new(|_| {});
		EventRegistry::register_onclick(
			"onclick",
			TreeLocation::default(),
			func,
		);
	}
}
