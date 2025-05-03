use wasm_bindgen::JsValue;

pub fn anyhow_to_jsvalue(e: anyhow::Error) -> JsValue {
	JsValue::from_str(&format!("{:?}", e))
}
