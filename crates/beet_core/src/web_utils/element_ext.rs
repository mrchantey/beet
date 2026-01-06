use wasm_bindgen::JsCast;
use web_sys::Element;
use web_sys::Node;

/// Element-focused DOM helpers in a cohesive, module-level API.
///
/// This module intentionally avoids extension traits so usage stays explicit:
/// - Good: `element_ext::query_selector(&root, ".child")`
/// - Bad: implement ad-hoc methods on foreign types
///
/// Design
/// - Functions are minimal, readable wrappers around `web_sys`.
/// - Errors in CSS selector parsing will panic with a clear message.
/// - Downcasting failures also panic; they usually indicate a misuse of the API.
///
/// Examples
/// Create and query children under a specific element (not the whole document).
/// ```ignore
/// use beet_core::web_utils::{document_ext as doc, element_ext};
/// use web_sys::HtmlParagraphElement;
///
/// // Build a parent and two children
/// let parent = doc::create_div();
/// let p1 = doc::create_paragraph();
/// p1.set_class_name("child");
/// p1.set_inner_html("first");
/// let p2 = doc::create_paragraph();
/// p2.set_class_name("child");
/// p2.set_inner_html("second");
///
/// // Attach children to parent and parent to the body
/// parent.append_child(&p1).unwrap();
/// parent.append_child(&p2).unwrap();
/// doc::append_child(&parent);
///
/// // Query just within `parent`
/// let found = element_ext::query_selector::<HtmlParagraphElement, _>(&parent, "p.child").unwrap();
/// assert_eq!(found.inner_html(), "first");
///
/// let all = element_ext::query_selector_all::<HtmlParagraphElement, _>(&parent, "p.child");
/// assert_eq!(all.len(), 2);
/// ```

/// Query a descendant of `root` and downcast to `T` if found.
///
/// Returns `None` if no element matches the selector.
pub fn query_selector<T, E>(root: &E, selector: &str) -> Option<T>
where
	T: JsCast,
	E: AsRef<Element>,
{
	root.as_ref()
		.query_selector(selector)
		.unwrap()
		.map(|el| el.dyn_into::<T>().unwrap())
}

/// Query all descendants of `root` and downcast each to `T`.
///
/// Returns an empty vector when no elements match.
pub fn query_selector_all<T, E>(root: &E, selector: &str) -> Vec<T>
where
	T: JsCast,
	E: AsRef<Element>,
{
	let list = root.as_ref().query_selector_all(selector).unwrap();
	let mut out = Vec::with_capacity(list.length() as usize);
	for i in 0..list.length() {
		let node = list.item(i).unwrap();
		out.push(node.dyn_into::<T>().unwrap());
	}
	out
}

/// Append `child` under `root`.
///
/// Panics on DOM errors.
pub fn append_child<E>(root: &E, child: &Node)
where
	E: AsRef<Element>,
{
	root.as_ref().append_child(child).unwrap();
}

/// Remove all children of `root`.
///
/// Panics on DOM errors.
pub fn clear_children<E>(root: &E)
where
	E: AsRef<Element>,
{
	let root = root.as_ref();
	while let Some(child) = root.first_child() {
		root.remove_child(&child).unwrap();
	}
}

#[cfg(test)]
#[cfg(target_arch = "wasm32")]
mod tests {
	use super::*;
	use crate::prelude::*;
	use crate::web_utils::document_ext as doc;
	use web_sys::HtmlDivElement;
	use web_sys::HtmlParagraphElement;

	#[test]
	#[ignore = "requires dom"]
	fn works() {
		// Existence checks to confirm DOM access
		let _ = doc::document();
		let _ = doc::body();
		let _ = doc::head();
	}

	#[ignore = "requires dom"]
	#[sweet::test]
	async fn works_async() {
		doc::clear_body();

		// Build a parent and two children
		let parent: HtmlDivElement = doc::create_div();
		parent.set_id("root");

		let p1: HtmlParagraphElement = doc::create_paragraph();
		p1.set_class_name("child");
		p1.set_inner_html("first");

		let p2: HtmlParagraphElement = doc::create_paragraph();
		p2.set_class_name("child");
		p2.set_inner_html("second");

		parent.append_child(&p1).unwrap();
		parent.append_child(&p2).unwrap();
		doc::append_child(&parent);

		// Query from the parent using top-level helpers
		let first =
			query_selector::<HtmlParagraphElement, _>(&parent, "p.child")
				.unwrap();
		first.inner_html().xpect_eq("first");

		let all =
			query_selector_all::<HtmlParagraphElement, _>(&parent, "p.child");
		all.len().xpect_eq(2usize);

		// Clean up parent children using our helper
		clear_children(&parent);
		let remaining =
			query_selector_all::<HtmlParagraphElement, _>(&parent, "p");
		remaining.is_empty().xpect_true();
	}
}
