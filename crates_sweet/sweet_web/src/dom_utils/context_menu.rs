use wasm_bindgen::prelude::*;
use web_sys::Event;

/// # Leaks
/// The closure is forgotten, only call this function once
pub fn prevent_context_menu() {
	let closure = Closure::wrap(Box::new(move |event: Event| {
		event.prevent_default();
	}) as Box<dyn FnMut(_)>);

	let document = web_sys::window().unwrap().document().unwrap();
	document
		.add_event_listener_with_callback(
			"contextmenu",
			closure.as_ref().unchecked_ref(),
		)
		.unwrap();

	closure.forget(); // Prevent the closure from being dropped prematurely
}
