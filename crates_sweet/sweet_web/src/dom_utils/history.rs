use wasm_bindgen::JsValue;
use web_sys::window;
use web_sys::Url;

pub struct History;

impl History {
	/// set search param without page reload
	pub fn set_flag(key: &str, value: bool) {
		if value {
			Self::set_param(key, "1");
		} else {
			Self::remove_param(key);
		}
	}
	pub fn set_param(key: &str, value: &str) {
		let url = window().unwrap().location().href().unwrap();
		let url = Url::new(&url).unwrap();
		let params = url.search_params();
		params.set(key, value);
		let url = url.href();
		Self::push(&url);
	}
	pub fn append_param(key: &str, value: &str) {
		let url = window().unwrap().location().href().unwrap();
		let url = Url::new(&url).unwrap();
		let params = url.search_params();
		params.append(key, value);
		let url = url.href();
		Self::push(&url);
	}

	pub fn remove_param(key: &str) {
		let url = window().unwrap().location().href().unwrap();
		let url = Url::new(&url).unwrap();
		let params = url.search_params();
		params.delete(key);
		let url = url.href();
		Self::push(&url);
	}

	pub fn push(path: &str) {
		window()
			.unwrap()
			.history()
			.unwrap()
			.push_state_with_url(&JsValue::UNDEFINED, "", Some(path))
			.unwrap();
	}
	pub fn push_preserve_params(path: &str) {
		let location = window().unwrap().location();
		let href = location.href().unwrap();
		let url = Url::new(&href).unwrap();
		url.set_pathname(path);
		Self::push(url.href().as_str());
	}
	pub fn replace(path: &str) {
		window()
			.unwrap()
			.history()
			.unwrap()
			.replace_state_with_url(&JsValue::UNDEFINED, path, Some(path))
			.unwrap();
	}

	// pub fn replace_params(params:UrlSearchParams){


	// }
}
