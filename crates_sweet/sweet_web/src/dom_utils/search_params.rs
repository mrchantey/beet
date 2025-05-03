use crate::*;
use web_sys::window;
use web_sys::UrlSearchParams;

pub struct SearchParams;

impl SearchParams {

	pub fn has(key: &str) -> bool {
		let search = window().unwrap().location().search().unwrap();
		let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
		params.has(key)
	}

	pub fn get(key: &str) -> Option<String> {
		let search = window().unwrap().location().search().unwrap();
		let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
		params.get(key)
	}
	pub fn get_all(key: &str) -> Vec<String> {
		let search = window().unwrap().location().search().unwrap();
		let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
		params.get_all(key).to_vec_str()
	}

	pub fn set(key: &str, value: &str) {
		if let Some(curr) = Self::get(key) {
			if curr == value {
				return;
			}
		}
		let loc = window().unwrap().location();
		let params = loc.search().unwrap();
		let params = UrlSearchParams::new_with_str(params.as_str()).unwrap();
		params.set(key, value);
		let params = params.to_string().as_string().unwrap();
		loc.set_search(params.as_str()).unwrap();
	}
	pub fn get_flag(key: &str) -> bool {
		match Self::get(key) {
			Some(val) => val != "0" && val != "false",
			None => false,
		}
	}
	pub fn set_flag(key: &str, val: bool) {
		if val {
			Self::set(key, "1");
		} else {
			Self::remove(key);
		}
	}

	pub fn remove(key: &str) {
		if let None = Self::get(key) {
			return;
		}
		let loc = window().unwrap().location();
		let search = loc.search().unwrap();
		let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
		params.delete(key);
		let params = params.to_string().as_string().unwrap();
		loc.set_search(params.as_str()).unwrap();
	}
}

pub fn path_name() -> String {
	window().unwrap().location().pathname().unwrap()
}

