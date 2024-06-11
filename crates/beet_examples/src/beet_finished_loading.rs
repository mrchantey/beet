pub fn beet_finished_loading() {
	#[cfg(target_arch = "wasm32")]
	wasm_funcs::finshed_loading();
}

#[cfg(target_arch = "wasm32")]
mod wasm_funcs {
	use forky_core::ResultTEExt;
	use js_sys::Array;
	use js_sys::Function;
	use js_sys::Reflect;
	use wasm_bindgen::JsCast;
	use wasm_bindgen::JsValue;


	pub fn finshed_loading() {
		let Ok(Some(el)) = web_sys::window()
			.unwrap()
			.document()
			.unwrap()
			.query_selector("beet-loading-canvas")
		else {
			return;
		};

		let Ok(func) = Reflect::get(&el, &JsValue::from_str("finishedLoading"))
		else {
			return;
		};

		let Ok(func) = func.dyn_into::<Function>() else {
			return;
		};

		Reflect::apply(&func, &el, &Array::new())
			.ok_or(|e| log::error!("{:?}", e));
	}
}
