//! DOM helpers for working with `web_sys::Document` in a cohesive, extension-free style.
//!
//! This module provides small, focused functions that wrap common DOM patterns.
//! Functions are named to read clearly at call-sites and return typed elements
//! whenever possible.
//!
//! Design:
//! - Prefer simple, ergonomic free functions over ad-hoc extension traits.
//! - Panic on missing `window`/`document` (tests run in a DOM-enabled wasm environment).
//! - Return `Result<_, JsValue>` only when interacting with DOM APIs that already return errors (e.g., creating script/link elements).
//!
//! Examples
//!
//! Create and append a div to the body:
//! ```
//! use beet_core::web_utils::document_ext::*;
//! let div = create_div();
//! div.set_inner_html("hello");
//! append_child(&div);
//! ```
//!
//! Find a typed element via CSS selector:
//! ```
//! use beet_core::web_utils::document_ext::*;
//! let div = create_div();
//! div.set_id("my-div");
//! append_child(&div);
//! let found = query_selector::<web_sys::HtmlDivElement>("#my-div").unwrap();
//! assert_eq!(found.id(), "my-div");
//! ```
//!
//! Add a style to the page head:
//! ```
//! use beet_core::web_utils::document_ext::*;
//! let link = add_style_src_to_head("data:text/css,body{display:block}").unwrap();
//! assert_eq!(link.rel(), "stylesheet");
//! ```

use crate::prelude::*;
use futures_lite::future::race;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::Document;
use web_sys::Event;
use web_sys::HtmlAnchorElement;
use web_sys::HtmlButtonElement;
use web_sys::HtmlCanvasElement;
use web_sys::HtmlDivElement;
use web_sys::HtmlElement;
use web_sys::HtmlHeadElement;
use web_sys::HtmlInputElement;
use web_sys::HtmlLinkElement;
use web_sys::HtmlParagraphElement;
use web_sys::HtmlScriptElement;
use web_sys::KeyboardEvent;
use web_sys::MouseEvent;
use web_sys::Node;
use web_sys::Window;

fn window() -> Window { web_sys::window().unwrap() }

/// Return the current document.
pub fn document() -> Document { window().document().unwrap() }

/// Return the `<head>` element of the current document.
pub fn head() -> HtmlHeadElement { document().head().unwrap() }

/// Return the `<body>` element of the current document.
pub fn body() -> HtmlElement { document().body().unwrap() }

/// Append a node to the end of `<body>`.
pub fn append_child(node: &Node) { body().append_child(node).unwrap(); }

/// Remove all children from `<body>`.
pub fn clear_body() {
	let body_el = body();
	while let Some(child) = body_el.first_child() {
		body_el.remove_child(&child).unwrap();
	}
}

/// Query the document and, if found, downcast the element to `T`.
///
/// Returns `None` if no element matches the selector.
pub fn query_selector<T>(selector: &str) -> Option<T>
where
	T: JsCast,
{
	document()
		.query_selector(selector)
		.unwrap()
		.map(|el| el.dyn_into::<T>().unwrap())
}

/// Create an `HtmlElement` with the provided tag name.
pub fn create_element(local_name: &str) -> HtmlElement {
	document()
		.create_element(local_name)
		.unwrap()
		.dyn_into()
		.unwrap()
}

/// Create a typed anchor element.
pub fn create_anchor() -> HtmlAnchorElement {
	document().create_element("a").unwrap().dyn_into().unwrap()
}

/// Create a typed canvas element.
pub fn create_canvas() -> HtmlCanvasElement {
	document()
		.create_element("canvas")
		.unwrap()
		.dyn_into()
		.unwrap()
}

/// Create a typed div element.
pub fn create_div() -> HtmlDivElement {
	document()
		.create_element("div")
		.unwrap()
		.dyn_into()
		.unwrap()
}

/// Create a typed input element.
pub fn create_input() -> HtmlInputElement {
	document()
		.create_element("input")
		.unwrap()
		.dyn_into()
		.unwrap()
}

/// Create a typed button element.
pub fn create_button() -> HtmlButtonElement {
	document()
		.create_element("button")
		.unwrap()
		.dyn_into()
		.unwrap()
}

/// Await the user's first interaction with the document (mousedown, scroll, or keydown).
/// Implemented using a Stream-based HtmlEventListener and racing the first next() without polling.
pub async fn await_interaction() {
	// Create three event streams on the window
	let mut on_click = HtmlEventListener::<MouseEvent>::new("mousedown");
	let mut on_scroll = HtmlEventListener::<Event>::new("scroll");
	let mut on_key = HtmlEventListener::<KeyboardEvent>::new("keydown");

	// Race the first event from any of the streams
	let click = async {
		on_click.next().await;
	};
	let scroll = async {
		on_scroll.next().await;
	};
	let key = async {
		on_key.next().await;
	};
	let _ = race(click, race(scroll, key)).await;
}


/// Create a typed paragraph element.
pub fn create_paragraph() -> HtmlParagraphElement {
	document().create_element("p").unwrap().dyn_into().unwrap()
}

/// Create a `<script src="...">` in the `<head>`.
///
/// Returns the created `HtmlScriptElement`.
pub fn add_script_src_to_head(src: &str) -> Result<HtmlScriptElement, JsValue> {
	let el = document()
		.create_element("script")?
		.dyn_into::<HtmlScriptElement>()?;
	el.set_src(src);
	el.set_type("text/javascript");
	head().append_child(&el)?;
	Ok(el)
}

/// Create a `<script>` with inline JS in the `<body>`.
///
/// Returns the created `HtmlScriptElement`.
pub fn add_script_content_to_body(
	code: &str,
) -> Result<HtmlScriptElement, JsValue> {
	let el = document()
		.create_element("script")?
		.dyn_into::<HtmlScriptElement>()?;
	el.set_type("text/javascript");
	el.set_inner_html(code);
	body().append_child(&el)?;
	Ok(el)
}

/// Create a `<link rel="stylesheet" href="...">` in the `<head>`.
///
/// Returns the created `HtmlLinkElement`.
pub fn add_style_src_to_head(src: &str) -> Result<HtmlLinkElement, JsValue> {
	let el = document()
		.create_element("link")?
		.dyn_into::<HtmlLinkElement>()?;
	el.set_href(src);
	el.set_rel("stylesheet");
	el.set_type("text/css");
	head().append_child(&el)?;
	Ok(el)
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use super::*;
	use sweet::prelude::*;

	#[ignore = "requires dom"]
	#[test]
	fn runs_in_wasm() {
		// This ensures the test harness can reach the DOM.
		let _ = document();
		let _ = head();
		let _ = body();
	}

	#[ignore = "requires dom"]
	#[test]
	fn creates_and_appends_div() {
		clear_body();

		let div = create_div();
		div.set_id("greeting");
		div.set_inner_html("hello");
		append_child(&div);

		let found = query_selector::<HtmlDivElement>("#greeting").unwrap();
		found.inner_html().xpect_eq("hello");
	}

	#[ignore = "requires dom"]
	#[sweet::test]
	async fn adds_script_and_style() {
		clear_body();
		let _script =
			add_script_content_to_body("window.__beet_flag = 1;").unwrap();
		let _style =
			add_style_src_to_head("data:text/css,body{outline:0}").unwrap();

		let script_el = query_selector::<HtmlScriptElement>("body script");
		let style_el =
			query_selector::<HtmlLinkElement>("head link[rel='stylesheet']");

		script_el.is_some().xpect_true();
		style_el.is_some().xpect_true();
	}
}
