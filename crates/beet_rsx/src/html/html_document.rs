use super::HtmlAttribute;
use super::HtmlElementNode;
use super::HtmlNode;
use super::RenderHtml;

/// A valid html document has a particular structure, if the
/// structure is missing it is usually inserterd by browsers, its
/// better to assert the structure before rendering for guaranteed
/// 1:1 mapping.
#[derive(Debug, Clone)]
pub struct HtmlDocument {
	pub head: Vec<HtmlNode>,
	pub body: Vec<HtmlNode>,
}

impl HtmlDocument {
	pub fn render_body(&self) -> String {
		let mut html = String::new();
		for node in &self.body {
			node.render_inner(&mut html);
		}
		html
	}

	pub fn insert_wasm_script(&mut self) {
		let script = r#"
		import init from './wasm/bindgen.js'
		init('./wasm/bindgen_bg.wasm')
			.catch((error) => {
				if (!error.message.startsWith("Using exceptions for control flow,"))
					throw error
			})
"#;
		self.body.push(HtmlNode::Element(HtmlElementNode {
			tag: "script".to_string(),
			self_closing: false,
			attributes: vec![HtmlAttribute {
				key: "type".to_string(),
				value: Some("module".to_string()),
			}],
			children: vec![HtmlNode::Text(script.to_string())],
		}));
	}

	/// Parses nodes, Appending them to the body unless they are one of
	/// the following in these positions:
	/// ```html
	/// <!DOCTYPE html>
	/// <html>
	/// 	<head>
	/// 		<!-- head nodes -->
	/// 	</head>
	/// 	<body>
	/// 		<!-- body nodes -->
	/// 	</body>
	/// </html>
	/// ```
	///
	/// Currently multiple head and body tags are supported
	/// but this is not guaranteed to be the case in the future
	pub fn from_nodes(nodes: Vec<HtmlNode>) -> Self {
		let mut head = vec![];
		let mut body = vec![];
		for node in nodes {
			match node.element_with_tag_owned("html") {
				Ok(el) => {
					for node in el.children {
						match node.element_with_tag_owned("head") {
							Ok(el) => {
								head.extend(el.children);
							}
							Err(other) => {
								match other.element_with_tag_owned("body") {
									Ok(el) => {
										body.extend(el.children);
									}
									Err(other) => body.push(other),
								}
							}
						}
					}
				}
				Err(other) => {
					if let HtmlNode::Doctype = other {
						continue;
					} else {
						body.push(other);
					}
				}
			}
		}
		Self { head, body }
	}
}

pub trait IntoHtmlDocument {
	fn into_document(self) -> HtmlDocument;
}
impl IntoHtmlDocument for Vec<HtmlNode> {
	fn into_document(self) -> HtmlDocument { HtmlDocument::from_nodes(self) }
}


impl RenderHtml for HtmlDocument {
	fn render_inner(&self, html: &mut String) {
		html.push_str("<!DOCTYPE html><html><head>");
		for node in &self.head {
			node.render_inner(html);
		}
		html.push_str("</head><body>");
		for node in &self.body {
			node.render_inner(html);
		}
		html.push_str("</body></html>");
	}
	fn render_pretty_inner(&self, html: &mut String, indent: &mut usize) {
		Self::push_pretty(html, indent, "<!DOCTYPE html>");
		Self::push_pretty(html, indent, "<html>");
		*indent += 1;
		Self::push_pretty(html, indent, "<head>");
		*indent += 1;
		for node in &self.head {
			node.render_pretty_inner(html, indent);
		}
		*indent -= 1;
		Self::push_pretty(html, indent, "</head>");
		Self::push_pretty(html, indent, "<body>");
		*indent += 1;
		for node in &self.body {
			node.render_pretty_inner(html, indent);
		}
		*indent -= 1;
		Self::push_pretty(html, indent, "</body>");
		*indent -= 1;
		Self::push_pretty(html, indent, "</html>");
	}
}


use std::vec;

pub struct HtmlDocumentIter<I> {
	head: I,
	body: I,
	is_head: bool,
}

impl IntoIterator for HtmlDocument {
	type Item = HtmlNode;
	type IntoIter = HtmlDocumentIter<vec::IntoIter<HtmlNode>>;

	fn into_iter(self) -> Self::IntoIter {
		HtmlDocumentIter {
			head: self.head.into_iter(),
			body: self.body.into_iter(),
			is_head: true,
		}
	}
}

impl<'a> IntoIterator for &'a HtmlDocument {
	type Item = &'a HtmlNode;
	type IntoIter = HtmlDocumentIter<std::slice::Iter<'a, HtmlNode>>;

	fn into_iter(self) -> Self::IntoIter {
		HtmlDocumentIter {
			head: self.head.iter(),
			body: self.body.iter(),
			is_head: true,
		}
	}
}

impl<'a> IntoIterator for &'a mut HtmlDocument {
	type Item = &'a mut HtmlNode;
	type IntoIter = HtmlDocumentIter<std::slice::IterMut<'a, HtmlNode>>;

	fn into_iter(self) -> Self::IntoIter {
		HtmlDocumentIter {
			head: self.head.iter_mut(),
			body: self.body.iter_mut(),
			is_head: true,
		}
	}
}

impl<I: Iterator> Iterator for HtmlDocumentIter<I> {
	type Item = I::Item;

	fn next(&mut self) -> Option<Self::Item> {
		if self.is_head {
			match self.head.next() {
				Some(item) => Some(item),
				None => {
					self.is_head = false;
					self.body.next()
				}
			}
		} else {
			self.body.next()
		}
	}
}

impl HtmlDocument {
	pub fn iter(&self) -> impl Iterator<Item = &HtmlNode> { self.into_iter() }

	pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut HtmlNode> {
		self.into_iter()
	}
}
