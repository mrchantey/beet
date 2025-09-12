//! Helpers for working with `HtmlIFrameElement` using a cohesive, module-style API.
//!
//! Design
//! - Small, explicit free functions (no extension traits).
//! - Async helpers leverage a Stream-based `HtmlEventListener` to await the next `load`.
//! - Clear docs, tests, and examples.
//!
//! Examples
//!
//! Reload and await load:
//! ```ignore
//! use beet_core::web_utils::{iframe, document_ext as doc};
//! use web_sys::HtmlIFrameElement;
//!
//! let iframe: HtmlIFrameElement = doc::document()
//!     .create_element("iframe").unwrap()
//!     .dyn_into().unwrap();
//!
//! doc::append_child(&iframe);
//! iframe::reload_async(&iframe).await;
//! ```
//!
//! Set source and await load:
//! ```ignore
//! use beet_core::web_utils::{iframe, document_ext as doc};
//! use web_sys::HtmlIFrameElement;
//!
//! let iframe: HtmlIFrameElement = doc::document()
//!     .create_element("iframe").unwrap()
//!     .dyn_into().unwrap();
//!
//! doc::append_child(&iframe);
//! iframe::set_source(&iframe, "data:text/html,<html><body>ok</body></html>").await;
//! // asserts same-origin document is present
//! assert!(iframe.content_document().is_some());
//! ```
use crate::web_utils::HtmlEventListener;
use wasm_bindgen::JsCast;
use web_sys::HtmlIFrameElement;

/// Reload the iframe synchronously (does not wait for load).
pub fn reload(iframe: &HtmlIFrameElement) {
	iframe
		.content_window()
		.unwrap()
		.location()
		.reload()
		.unwrap();
}

/// Reload the iframe and await the next `load` event.
///
/// This uses a Stream-based event listener to await a single `load`.
pub async fn reload_async(iframe: &HtmlIFrameElement) {
	reload(iframe);
	wait_for_load(iframe).await;
}

/// Set the iframe `src` and await the next `load` event.
///
/// Panics if the iframe finishes loading but `content_document()` is `None`,
/// which commonly indicates a cross-origin restriction.
pub async fn set_source(iframe: &HtmlIFrameElement, url: &str) {
	iframe.set_src(url);
	wait_for_load(iframe).await;
	if iframe.content_document().is_none() {
		panic!(
			"iframe loaded src: {url}\nbut content_document is None (likely a CORS issue)"
		);
	}
}

/// Await the next `load` event for the provided iframe.
pub async fn wait_for_load(iframe: &HtmlIFrameElement) {
	let target = iframe.clone().unchecked_into::<web_sys::EventTarget>();
	let mut loads =
		HtmlEventListener::<web_sys::Event>::new_with_target("load", target);
	// Await exactly one event
	let _ = loads.next_event().await;
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use super::*;
	use crate::web_utils::document_ext as doc;
	use sweet::prelude::*;

	#[ignore = "requires dom"]
	#[test]
	fn works() {
		// DOM access smoke-check
		let _ = doc::document();
		let _ = doc::head();
		let _ = doc::body();
	}

	#[ignore = "requires dom"]
	#[sweet::test]
	async fn works_async() {
		doc::clear_body();

		// Create and attach an iframe, then set a same-origin data URL and await load
		let iframe: HtmlIFrameElement = doc::document()
			.create_element("iframe")
			.unwrap()
			.dyn_into()
			.unwrap();

		// Make it visible to avoid some headless environment quirks


		doc::append_child(&iframe);

		let data_url = "data:text/html,<html><body>ok</body></html>";
		set_source(&iframe, data_url).await;

		iframe.content_document().is_some().xpect_true();

		// Exercise reload path too
		reload_async(&iframe).await;
		iframe.content_document().is_some().xpect_true();
	}
}
