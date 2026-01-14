//! Query-string utilities exposed as a cohesive, explicit module: `search_params_ext`.
//!
//! Design
//! - Explicit, module-style free functions (no ad-hoc extension traits).
//! - Read operations parse the current `window.location.search`.
//! - Write operations update the address bar without page reload using History API.
//! - Wasm-friendly tests included; doc examples are illustrative and may be ignored by doctests.
//!
//! Examples
//!
//! Read values:
//! ```ignore
//! use beet_core::web_utils::search_params_ext;
//!
//! let val = search_params_ext::get("color");
//! let is_debug = search_params_ext::get_flag("debug");
//! ```
//!
//! Write values without reloading the page:
//! ```ignore
//! use beet_core::web_utils::search_params_ext;
//!
//! search_params_ext::set("color", "red");
//! search_params_ext::set_flag("debug", true);
//! search_params_ext::remove("color");
//! ```

use wasm_bindgen::JsValue;
use web_sys::Url;
use web_sys::UrlSearchParams;


fn current_window() -> web_sys::Window { web_sys::window().unwrap() }

fn current_url() -> Url {
	let href = current_window().location().href().unwrap();
	Url::new(&href).unwrap()
}

fn replace_url(url: &Url) {
	let history = current_window().history().unwrap();
	history
		.replace_state_with_url(&JsValue::UNDEFINED, "", Some(&url.href()))
		.unwrap();
}

/// Whether the query string contains `key`.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// let _ = search_params_ext::has("debug");
/// ```
pub fn has(key: &str) -> bool {
	let search = current_window().location().search().unwrap();
	let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
	params.has(key)
}

/// Get the first value for `key` from the query string.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// let maybe_color = search_params_ext::get("color");
/// ```
pub fn get(key: &str) -> Option<String> {
	let search = current_window().location().search().unwrap();
	let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
	params.get(key)
}

/// Get all values for `key` from the query string.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// let all = search_params_ext::get_all("tag");
/// ```
pub fn get_all(key: &str) -> Vec<String> {
	let search = current_window().location().search().unwrap();
	let params = UrlSearchParams::new_with_str(search.as_str()).unwrap();
	params
		.get_all(key)
		.iter()
		.map(|v| v.as_string().unwrap())
		.collect()
}

/// Set `key=value` in the query string without reloading the page.
/// If the key already exists, its first value is replaced (mirrors `URLSearchParams.set` semantics).
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// search_params_ext::set("color", "red");
/// ```
pub fn set(key: &str, value: &str) {
	// No-op if unchanged
	if let Some(curr) = get(key) {
		if curr == value {
			return;
		}
	}

	let url = current_url();
	let params = url.search_params();
	params.set(key, value);
	replace_url(&url);
}

/// Interpret the presence of `key` as a boolean flag.
/// Returns true for values other than "0" or "false".
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// let debug = search_params_ext::get_flag("debug");
/// ```
pub fn get_flag(key: &str) -> bool {
	match get(key) {
		Some(val) => val != "0" && val.to_ascii_lowercase() != "false",
		None => false,
	}
}

/// Set or remove a boolean flag in the query string without reloading the page.
/// When `true`, sets `key=1`; when `false`, removes `key`.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// search_params_ext::set_flag("debug", true);
/// search_params_ext::set_flag("debug", false);
/// ```
pub fn set_flag(key: &str, val: bool) {
	if val {
		set(key, "1");
	} else {
		remove(key);
	}
}

/// Remove `key` from the query string without reloading the page.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// search_params_ext::remove("color");
/// ```
pub fn remove(key: &str) {
	// No-op if key not present
	if get(key).is_none() {
		return;
	}
	let url = current_url();
	let params = url.search_params();
	params.delete(key);
	replace_url(&url);
}

/// Return the current `location.pathname`.
///
/// Examples
/// ```ignore
/// use beet_core::web_utils::search_params_ext;
/// let path = search_params_ext::path_name();
/// ```
pub fn path_name() -> String { current_window().location().pathname().unwrap() }

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use crate::prelude::*;
	use wasm_bindgen::JsValue;
	use web_sys::Url;
	use web_sys::window;

	fn set_url_query(qs: &str) {
		let win = window().unwrap();
		let href = win.location().href().unwrap();
		let url = Url::new(&href).unwrap();
		url.set_search(qs);
		win.history()
			.unwrap()
			.replace_state_with_url(&JsValue::UNDEFINED, "", Some(&url.href()))
			.unwrap();
	}

	#[test]
	#[ignore = "requires dom"]
	fn works() {
		// Smoke check: ensure we can reach basic DOM APIs.
		let _ = window().unwrap();
	}

	#[crate::test]
	#[ignore = "requires dom"]
	async fn read_write_params() {
		// Start from a known baseline
		set_url_query("");

		// Write: set and read back
		search_params_ext::set("color", "red");
		search_params_ext::get("color").xpect_eq(Some("red".to_string()));

		// Flags
		search_params_ext::set_flag("debug", true);
		search_params_ext::get_flag("debug").xpect_true();

		// Update and remove
		search_params_ext::set("color", "blue");
		search_params_ext::get("color").xpect_eq(Some("blue".to_string()));

		search_params_ext::remove("color");
		search_params_ext::get("color").xpect_eq(None);

		// Multiple values (manually construct)
		set_url_query("?tag=a&tag=b&tag=c");
		let tags = search_params_ext::get_all("tag");
		tags.len().xpect_eq(3usize);
		tags[0].xpect_eq("a".to_string());
		tags[1].xpect_eq("b".to_string());
		tags[2].xpect_eq("c".to_string());
	}
}
