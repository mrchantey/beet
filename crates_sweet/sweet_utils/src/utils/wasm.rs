use std::time::Duration;
use wasm_bindgen::prelude::*;
use web_sys::window;

pub struct TimeoutHandle {
	_closure: Closure<dyn Fn()>,
	handle: i32,
}

impl Drop for TimeoutHandle {
	fn drop(&mut self) {
		if let Some(window) = window() {
			window.clear_timeout_with_handle(self.handle);
		}
	}
}

pub fn set_timeout_ms<F>(ms: u64, f: F)
where
	F: Fn() + 'static,
{
	set_timeout(Duration::from_millis(ms), f)
}


pub fn set_timeout<F>(duration: Duration, f: F)
where
	F: Fn() + 'static,
{
	let window = window().unwrap();
	let closure = Closure::wrap(Box::new(f) as Box<dyn Fn()>);
	let _handle = window
		.set_timeout_with_callback_and_timeout_and_arguments_0(
			closure.as_ref().unchecked_ref(),
			duration.as_millis() as i32,
		)
		.unwrap();
	closure.forget();
}
