//! Client-side support for invoking server actions.
//!
//! Generated client-action callers build their requests via
//! [`server_action_request`], which prepends the globally configured server URL
//! to the action path. The default is the local dev server on native and the
//! current page origin on wasm; override it with [`set_server_url`].

use beet_net::prelude::*;
use std::sync::LazyLock;
use std::sync::Mutex;

/// The base URL prepended to server-action paths.
static SERVER_URL: LazyLock<Mutex<Url>> = LazyLock::new(|| {
	#[cfg(not(target_arch = "wasm32"))]
	let raw = DEFAULT_SERVER_LOCAL_URL.to_string();
	#[cfg(target_arch = "wasm32")]
	let raw = beet_core::exports::web_sys::window()
		.and_then(|window| window.location().origin().ok())
		.unwrap_or_else(|| DEFAULT_SERVER_LOCAL_URL.to_string());
	Mutex::new(Url::parse(raw))
});

/// Returns the currently configured server URL for client actions.
pub fn server_url() -> Url { SERVER_URL.lock().unwrap().clone() }

/// Sets the server URL used by all subsequent client-action calls.
pub fn set_server_url(url: impl Into<Url>) {
	*SERVER_URL.lock().unwrap() = url.into();
}

/// Builds a [`Request`] to a server-action path using the configured
/// [`server_url`].
pub fn server_action_request(method: HttpMethod, path: &str) -> Request {
	let base = server_url().to_string();
	let url = format!("{}{}", base.trim_end_matches('/'), path);
	Request::new(method, url)
}
