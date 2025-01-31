use crate::html::HtmlDocument;
use crate::html::HtmlNode;
use web_sys::Element;



pub struct DomMounter;



impl DomMounter {
	/// by deffault the dom mounter will not collapse text nodes
	/// this recursively collapses text nodes into their parent element
	pub fn normalize() {
		web_sys::window().unwrap().document().unwrap().normalize();
	}


	pub fn mount_doc(html_doc: &HtmlDocument) {
		let dom_doc = web_sys::window().unwrap().document().unwrap();
		let head = dom_doc.head().unwrap().into();
		Self::mount_nodes(&dom_doc, &head, &html_doc.head);
		let body = dom_doc.body().unwrap().into();
		Self::mount_nodes(&dom_doc, &body, &html_doc.body);
	}
	pub fn mount_nodes(
		dom_doc: &web_sys::Document,
		parent: &Element,
		nodes: &Vec<HtmlNode>,
	) {
		for node in nodes {
			Self::mount_node(dom_doc, parent, node);
		}
	}
	pub fn mount_node(
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
