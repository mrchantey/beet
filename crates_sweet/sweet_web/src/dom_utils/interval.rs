use crate::ClosureFnMutT2Ext;
use js_sys::Function;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use web_sys::window;
use web_sys::Window;

pub struct Interval {
	_func: Function,
	target: Window,
	handle: i32,
}

impl Drop for Interval {
	fn drop(&mut self) { self.target.clear_interval_with_handle(self.handle); }
}


impl Interval {
	#[must_use]
	pub fn new(interval: i32, func: impl 'static + Fn()) -> Self {
		Self::new_with_target(interval, window().unwrap(), func)
	}
	#[must_use]
	pub fn new_with_target(
		interval: i32,
		target: Window,
		func: impl 'static + FnMut(),
	) -> Self {
		let func: Function = Closure::from_func_no_args(func)
			.into_js_value()
			.unchecked_into();

		let handle = target
			.set_interval_with_callback_and_timeout_and_arguments_0(
				&func,
				interval as i32,
			)
			.unwrap();

		Self {
			_func: func,
			target,
			handle,
		}
	}

	pub fn forget(self) { std::mem::forget(self) }
}
