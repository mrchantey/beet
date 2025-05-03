use js_sys::Function;
use js_sys::Reflect;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

pub struct LogBuffer {
	name: &'static str,
	log: Rc<RefCell<String>>,
	// _closure: Closure<dyn FnMut(JsValue)>,
	func: Function,
}

impl LogBuffer {
	// this breaks panics
	// pub fn new_err() -> Self { Self::new("error") }
	pub fn new_log() -> Self { Self::new("log") }
	pub fn get_log(&self) -> std::cell::RefMut<'_, String> {
		self.log.borrow_mut()
	}
	pub fn new(name: &'static str) -> Self {
		let log = Rc::new(RefCell::new(String::new()));
		let log2 = log.clone();

		let closure: Closure<dyn FnMut(JsValue)> =
			Closure::new(move |val: JsValue| {
				if let Some(mut val) = val.as_string() {
					val.push('\n');
					log2.borrow_mut().push_str(val.as_str());
				}
			});
		let func = Self::get_func(name);
		Self::set_func(name, closure.as_ref().unchecked_ref());

		Self {
			name,
			log,
			// _closure,
			func,
		}
	}
	fn get_func(name: &str) -> Function {
		let window = web_sys::window().unwrap();
		let console = Reflect::get(&window, &"console".into()).unwrap();
		let func = Reflect::get(&console, &name.into()).unwrap();
		func.into()
	}
	fn set_func(name: &str, func: &JsValue) {
		let window = web_sys::window().unwrap();
		let console = Reflect::get(&window, &"console".into()).unwrap();
		Reflect::set(&console, &name.into(), func).unwrap();
	}

	pub fn end(self) -> String { (*self.log.borrow()).clone() }
}

impl Drop for LogBuffer {
	fn drop(&mut self) { Self::set_func(self.name, &self.func.clone().into()); }
}
