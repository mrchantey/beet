use crate::ClosureFnMutT1T2Ext;
use js_sys::Function;
use js_sys::Reflect;
use wasm_bindgen::convert::FromWasmAbi;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;

#[derive(Debug, Clone)]
pub struct ReplaceFunc {
	name: &'static str,
	parent: JsValue,
	prev_func: Function,
}

impl ReplaceFunc {
	pub fn func(parent: JsValue, name: &'static str) -> Function {
		Reflect::get(&parent, &name.into()).unwrap().into()
	}
	pub fn new<T>(
		parent: JsValue,
		name: &'static str,
		func: impl FnMut(T) + 'static,
	) -> Self
	where
		T: FromWasmAbi + 'static,
	{
		let closure = Closure::from_func(func);
		let prev_func: Function =
			Reflect::get(&parent, &name.into()).unwrap().into();
		Reflect::set(&parent, &name.into(), closure.as_ref().unchecked_ref())
			.unwrap();

		Self {
			name,
			parent,
			prev_func,
		}
	}
}

impl Drop for ReplaceFunc {
	fn drop(&mut self) {
		Reflect::set(&self.parent, &self.name.into(), &self.prev_func).unwrap();
	}
}
