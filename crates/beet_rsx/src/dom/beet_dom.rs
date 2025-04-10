use crate::prelude::*;
use web_sys::Element;



#[deprecated = "we now use client directives exclusively"]
pub struct BeetDom;


#[allow(deprecated)]
impl BeetDom {
	pub fn mount(app: impl 'static + Fn() -> RsxNode) {
		use sweet::prelude::wasm::set_timeout_ms;

		let doc = app().xpipe(RsxToHtmlDocument::default()).unwrap();

		// effects are called on render
		Self::mount_doc(&doc);
		Self::normalize();
		// give the dom a moment to mount
		set_timeout_ms(100, move || {
			Self::hydrate(app());
		});
	}

	pub fn hydrate<M>(app: impl IntoRsxNode<M>) {
		DomTarget::set(BrowserDomTarget::default());
		// effects called here too
		app.into_node().xpipe(RegisterEffects::default()).unwrap();
		EventRegistry::initialize().unwrap();
	}

	/// by default the dom mounter will not collapse text nodes
	/// this recursively collapses text nodes into their parent element
	fn normalize() {
		web_sys::window().unwrap().document().unwrap().normalize();
	}


	fn mount_doc(html_doc: &HtmlDocument) {
		let dom_doc = web_sys::window().unwrap().document().unwrap();
		let head = dom_doc.head().unwrap().into();
		Self::mount_nodes(&dom_doc, &head, &html_doc.head);
		let body = dom_doc.body().unwrap().into();
		Self::mount_nodes(&dom_doc, &body, &html_doc.body);
	}
	fn mount_nodes(
		dom_doc: &web_sys::Document,
		parent: &Element,
		nodes: &Vec<HtmlNode>,
	) {
		for node in nodes {
			Self::mount_node(dom_doc, parent, node);
		}
	}
	fn mount_node(
		dom_doc: &web_sys::Document,
		parent: &Element,
		node: &HtmlNode,
	) {
		// sweet_utils::log!("parent {:?}", parent);
		match node {
			HtmlNode::Doctype => {}
			HtmlNode::Comment(comment) => {
				// sweet_utils::log!("comment node: {}", comment);
				let comment = dom_doc.create_comment(comment);
				parent.append_child(&comment).expect("pizza");
			}
			HtmlNode::Text(text) => {
				// sweet_utils::log!("text node: {}", text);
				let text = dom_doc.create_text_node(text);
				parent.append_child(&text).expect("pizza");
			}
			HtmlNode::Element(html_el) => {
				// sweet_utils::log!("element: {}", html_el.tag);
				let dom_el = dom_doc.create_element(&html_el.tag).unwrap();
				for attr in html_el.attributes.iter() {
					dom_el
						.set_attribute(
							&attr.key,
							&attr.value.as_deref().unwrap_or_default(),
						)
						.unwrap();
				}
				parent.append_child(&dom_el).unwrap();
				Self::mount_nodes(dom_doc, &dom_el, &html_el.children);
			}
		}
	}
}
