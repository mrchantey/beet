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
//! Set a same-origin srcdoc and await load:
//! ```ignore
//! use beet_core::web_utils::{iframe, document_ext as doc};
//! use web_sys::HtmlIFrameElement;
//!
//! let iframe: HtmlIFrameElement = doc::document()
//!     .create_element("iframe").unwrap()
//!     .dyn_into().unwrap();
//!
//! doc::append_child(&iframe);
//! iframe::set_srcdoc(&iframe, "<html><body>ok</body></html>").await;
//! // srcdoc inherits the parent origin, so the document is reachable
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
/// A loaded frame does not imply a reachable document:
/// `content_document()` is `None` for any cross-origin frame, including every
/// `data:` url (opaque origin). For a same-origin scripted frame use
/// [`set_srcdoc`], which inherits the parent origin and verifies arrival.
///
/// A freshly-inserted frame may still deliver its initial `about:blank` load
/// after this attaches, in which case that is the event awaited here; a frame
/// that has settled awaits the real one.
pub async fn set_source(iframe: &HtmlIFrameElement, url: &str) {
	let target = iframe.clone().unchecked_into::<web_sys::EventTarget>();
	let mut loads =
		HtmlEventListener::<web_sys::Event>::new_with_target("load", target);
	// a present srcdoc takes precedence over src, making the assignment a
	// silent no-op that never fires load. Assign src first, then remove:
	// removing srcdoc navigates to the src attribute, so this order yields
	// exactly one navigation (remove-first bounces through about:blank, and a
	// single load await would resolve on the wrong document)
	iframe.set_src(url);
	iframe.remove_attribute("srcdoc").ok();
	let _ = loads.next_event().await;
}

/// Set the iframe `srcdoc` and await the srcdoc document being current and
/// complete. Unlike a `data:` [`set_source`], a srcdoc frame inherits the
/// parent origin, so `content_document()` is reachable afterwards.
///
/// Consumes load events until the srcdoc document has actually arrived: a
/// freshly-inserted frame's initial `about:blank` load can land after the
/// srcdoc assignment, so a single awaited event proves nothing.
pub async fn set_srcdoc(iframe: &HtmlIFrameElement, html: &str) {
	let target = iframe.clone().unchecked_into::<web_sys::EventTarget>();
	let mut loads =
		HtmlEventListener::<web_sys::Event>::new_with_target("load", target);
	iframe.set_srcdoc(html);
	loop {
		let arrived = iframe
			.content_document()
			.map(|doc| {
				doc.url().unwrap_or_default() == "about:srcdoc"
					&& doc.ready_state() == "complete"
			})
			.unwrap_or(false);
		if arrived {
			return;
		}
		let _ = loads.next_event().await;
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
	use crate::prelude::*;
	use crate::web_utils::document_ext as doc;

	#[crate::test(browser)]
	fn works() {
		// DOM access smoke-check
		let _ = doc::document();
		let _ = doc::head();
		let _ = doc::body();
	}

	#[crate::test(browser)]
	async fn works_async() {
		let iframe: HtmlIFrameElement = doc::document()
			.create_element("iframe")
			.unwrap()
			.dyn_into()
			.unwrap();
		doc::append_child(&iframe);

		// a srcdoc frame inherits the parent origin, so the document is
		// reachable and carries the content
		set_srcdoc(&iframe, "<html><body>ok</body></html>").await;
		iframe
			.content_document()
			.unwrap()
			.body()
			.unwrap()
			.text_content()
			.unwrap()
			.xpect_eq("ok");

		// the reload path resolves and keeps the document reachable
		reload_async(&iframe).await;
		iframe.content_document().is_some().xpect_true();

		// a `data:` src loads fine but is an opaque origin: no document access
		set_source(&iframe, "data:text/html,<html><body>ok</body></html>")
			.await;
		iframe.content_document().is_none().xpect_true();
	}
}
