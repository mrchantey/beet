use crate::prelude::*;
use anyhow::Result;

#[derive(Default)]
pub struct HtmlToDocument;

impl RsxPlugin<Vec<HtmlNode>, HtmlDocument> for HtmlToDocument {
	fn apply(self, value: Vec<HtmlNode>) -> Result<HtmlDocument> {
		Ok(HtmlDocument::from_nodes(value))
	}
}

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

	/// Create a Vec<HtmlNode> that has a valid html document layout
	pub fn into_nodes(self) -> Vec<HtmlNode> {
		vec![
			HtmlNode::Doctype,
			HtmlNode::Element(HtmlElementNode {
				tag: "html".to_string(),
				self_closing: false,
				attributes: vec![],
				children: vec![
					HtmlNode::Element(HtmlElementNode {
						tag: "head".to_string(),
						self_closing: false,
						attributes: vec![],
						children: self.head,
					}),
					HtmlNode::Element(HtmlElementNode {
						tag: "body".to_string(),
						self_closing: false,
						attributes: vec![],
						children: self.body,
					}),
				],
			}),
		]
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
