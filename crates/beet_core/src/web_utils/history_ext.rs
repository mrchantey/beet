//! Helpers for manipulating the browser URL and history without page reloads.
//!
//! These utilities:
//! - Use the History API (`pushState`, `replaceState`) to update the address bar.
//! - Preserve single-page app state (no full navigation).
//! - Provide convenience functions for query string management.
//!
//! See also: the `location` helpers for direct navigation that does trigger a load/replace.
use super::history_ext;
use wasm_bindgen::JsValue;
use web_sys::Url;
use web_sys::window;
/// Set or remove a boolean flag in the query string without reloading the page.
///
/// When `value` is true, sets `key=1`. When false, removes `key`.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::set_flag("debug", true);
/// history_ext::set_flag("debug", false);
/// ```
pub fn set_flag(key: &str, value: bool) {
	if value {
		history_ext::set_param(key, "1");
	} else {
		history_ext::remove_param(key);
	}
}
/// Set `key=value` in the current URL's query string without reloading the page.
///
/// This preserves other parameters and pushes a new history entry.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::set_param("color", "red");
/// ```
pub fn set_param(key: &str, value: &str) {
	let url = window().unwrap().location().href().unwrap();
	let url = Url::new(&url).unwrap();
	let params = url.search_params();
	params.set(key, value);
	let url = url.href();
	history_ext::push(&url);
}
/// Append a value to `key` in the query string, preserving existing values.
///
/// This mirrors `URLSearchParams.append` and pushes a new history entry.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::append_param("tag", "a");
/// history_ext::append_param("tag", "b");
/// ```
pub fn append_param(key: &str, value: &str) {
	let url = window().unwrap().location().href().unwrap();
	let url = Url::new(&url).unwrap();
	let params = url.search_params();
	params.append(key, value);
	let url = url.href();
	history_ext::push(&url);
}

/// Remove `key` and all associated values from the query string and push a new history entry.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::remove_param("color");
/// ```
pub fn remove_param(key: &str) {
	let url = window().unwrap().location().href().unwrap();
	let url = Url::new(&url).unwrap();
	let params = url.search_params();
	params.delete(key);
	let url = url.href();
	history_ext::push(&url);
}

/// Push a new URL into the browser history without reloading the page.
///
/// `path` can be a relative path or a full URL. Use this to navigate while preserving SPA state.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::push("/dashboard?tab=home");
/// ```
pub fn push(path: &str) {
	window()
		.unwrap()
		.history()
		.unwrap()
		.push_state_with_url(&JsValue::UNDEFINED, "", Some(path))
		.unwrap();
}
/// Push a new path while preserving the current query parameters.
///
/// For example, calling with `"/settings"` on `"/page?debug=1"` results in `"/settings?debug=1"`.
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::push_preserve_params("/settings");
/// ```
pub fn push_preserve_params(path: &str) {
	let location = window().unwrap().location();
	let href = location.href().unwrap();
	let url = Url::new(&href).unwrap();
	url.set_pathname(path);
	history_ext::push(url.href().as_str());
}
/// Replace the current history entry with `path` without reloading the page.
///
/// This updates the URL in-place (no new history entry).
///
/// Examples
/// ```ignore
/// use beet_core::prelude::*;
/// history_ext::replace("/login");
/// ```
pub fn replace(path: &str) {
	window()
		.unwrap()
		.history()
		.unwrap()
		.replace_state_with_url(&JsValue::UNDEFINED, path, Some(path))
		.unwrap();
}
