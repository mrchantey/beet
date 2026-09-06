//! JavaScript conversion for [`Value`].
use crate::prelude::*;

impl Value {
	/// Convert into a live [`wasm_bindgen::JsValue`] for binding into a wasm script
	/// host, the wasm analogue of the native runtimes' input marshalling.
	///
	/// Mirrors the shape `JSON.parse` of the native JSON encoding would yield:
	/// numbers (incl. bytes) become JS numbers, [`Value::Bytes`] an array of byte
	/// numbers, a [`Value::List`] an array, and a [`Value::Map`] an object with
	/// string keys.
	pub fn to_js_value(&self) -> wasm_bindgen::JsValue {
		use wasm_bindgen::JsValue;
		match self {
			Value::Null => JsValue::NULL,
			Value::Bool(bool) => JsValue::from_bool(*bool),
			Value::Int(int) => JsValue::from_f64(*int as f64),
			Value::Uint(uint) => JsValue::from_f64(*uint as f64),
			Value::Float(float) => JsValue::from_f64(*float),
			Value::Str(str) => JsValue::from_str(str),
			Value::Bytes(bytes) => bytes
				.iter()
				.map(|byte| JsValue::from_f64(*byte as f64))
				.collect::<js_sys::Array>()
				.into(),
			Value::List(list) => list
				.iter()
				.map(Value::to_js_value)
				.collect::<js_sys::Array>()
				.into(),
			Value::Map(map) => {
				let object = js_sys::Object::new();
				for (key, value) in map {
					js_sys::Reflect::set(
						&object,
						&JsValue::from_str(key.as_str()),
						&value.to_js_value(),
					)
					.ok();
				}
				object.into()
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use wasm_bindgen::JsValue;

	#[crate::test]
	fn preserves_js_representation() {
		let value = value!({
			"name": "Ada",
			"items": [1, 2]
		});
		let js_value = value.to_js_value();

		js_sys::Reflect::get(&js_value, &JsValue::from_str("name"))
			.unwrap()
			.as_string()
			.unwrap()
			.xpect_eq("Ada".to_string());
		let items = js_sys::Array::from(
			&js_sys::Reflect::get(&js_value, &JsValue::from_str("items"))
				.unwrap(),
		);
		items.length().xpect_eq(2);
		items.get(0).as_f64().unwrap().xpect_eq(1.0);
		items.get(1).as_f64().unwrap().xpect_eq(2.0);
	}
}
