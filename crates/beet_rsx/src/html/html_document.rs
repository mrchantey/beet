use super::HtmlNode;
use super::RenderHtml;




/// Requirements for a valid html document are that
/// 1. head and body tags are present

pub struct HtmlDocument {
	pub head: Vec<HtmlNode>,
	pub body: Vec<HtmlNode>,
}

impl HtmlDocument {
	pub fn render_body(&self) -> String {
		let mut html = String::new();
		for node in &self.body {
			node.render_html_with_buf(&mut html);
		}
		html
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
	fn render_html_with_buf(&self, html: &mut String) {
		html.push_str("<!DOCTYPE html><html><head>");
		for node in &self.head {
			node.render_html_with_buf(html);
		}
		html.push_str("</head><body>");
		for node in &self.body {
			node.render_html_with_buf(html);
		}
		html.push_str("</body></html>");
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
