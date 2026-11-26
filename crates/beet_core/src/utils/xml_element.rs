//! [`XmlNode`] and [`XmlElement`], an xml document read as a plain tree.
use crate::prelude::*;

/// A node of an xml document as [`XmlNode::parse_document`] reads it: an
/// element or its text, with comments, processing instructions and the
/// doctype dropped.
///
/// What a protocol or a file format answers in xml reads this way, ie an S3
/// listing or a Word document's parts, and never a scene: markup that
/// becomes entities is BSX's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlNode {
	/// An element.
	Element(XmlElement),
	/// Text with its references decoded, adjacent text and CDATA joined into
	/// one.
	Text(String),
}

/// An xml element: its tag as written, prefix and all, its attributes in
/// order and its children.
///
/// A name is matched by its local part, whatever prefix it was written with,
/// and a prefix is never resolved against a namespace, so a document is read
/// by [`child`](Self::child) and [`text_at`](Self::text_at) rather than a
/// schema, and a field a service adds later is simply not looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmlElement {
	/// The tag as written, ie `w:p`.
	pub tag: SmolStr,
	/// The attributes in order.
	pub attributes: Vec<XmlAttribute>,
	/// The children in order.
	pub children: Vec<XmlNode>,
}

/// An attribute of an [`XmlElement`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttribute {
	/// The key as written, ie `r:id`.
	pub key: SmolStr,
	/// The value with its references decoded, a literal tab or line break
	/// the space xml normalizes it to.
	pub value: String,
}

impl XmlNode {
	/// The nodes of `xml`, its document element among them.
	///
	/// ## Errors
	/// Errors naming `line:col` when an element is never closed, a closing tag
	/// names another, or a construct is cut off.
	pub fn parse_document(xml: &str) -> Result<Vec<XmlNode>> {
		XmlReader { xml, offset: 0 }.document()
	}

	/// The first element of a parsed document, its document element.
	pub fn document_element(nodes: &[XmlNode]) -> Option<&XmlElement> {
		nodes.iter().find_map(|node| match node {
			XmlNode::Element(element) => Some(element),
			XmlNode::Text(_) => None,
		})
	}
}

impl XmlElement {
	/// The document element of `xml`.
	///
	/// ## Errors
	/// Errors when `xml` does not read ([`XmlNode::parse_document`]) or holds
	/// no element.
	pub fn parse(xml: &str) -> Result<Self> {
		XmlNode::parse_document(xml)?
			.into_iter()
			.find_map(|node| match node {
				XmlNode::Element(element) => Some(element),
				XmlNode::Text(_) => None,
			})
			.ok_or_else(|| bevyhow!("the xml has no root element"))
	}

	/// The tag without its prefix, ie `p` for `w:p`.
	pub fn local_name(&self) -> &str { Self::local(&self.tag) }

	/// The element children, in order.
	pub fn elements(&self) -> impl Iterator<Item = &XmlElement> {
		self.children.iter().filter_map(|node| match node {
			XmlNode::Element(element) => Some(element),
			XmlNode::Text(_) => None,
		})
	}

	/// The element children whose local name is `local`, in order.
	pub fn children_named<'a>(
		&'a self,
		local: &'a str,
	) -> impl Iterator<Item = &'a XmlElement> {
		self.elements()
			.filter(move |element| element.local_name() == local)
	}

	/// The first element child whose local name is `local`.
	pub fn child(&self, local: &str) -> Option<&XmlElement> {
		self.elements()
			.find(|element| element.local_name() == local)
	}

	/// The element at the `/` separated `path` of local names below this one.
	pub fn at(&self, path: &str) -> Option<&XmlElement> {
		path.split('/')
			.filter(|name| !name.is_empty())
			.try_fold(self, |element, name| element.child(name))
	}

	/// The first element whose local name is `local` anywhere below this one,
	/// depth first.
	pub fn find(&self, local: &str) -> Option<&XmlElement> {
		self.elements()
			.find_map(|child| match child.local_name() == local {
				true => Some(child),
				false => child.find(local),
			})
	}

	/// Every descendant element whose local name is `local`, in document
	/// order.
	pub fn descendants_named(&self, local: &str) -> Vec<&XmlElement> {
		let mut out = Vec::new();
		self.collect_named(local, &mut out);
		out
	}

	fn collect_named<'a>(&'a self, local: &str, out: &mut Vec<&'a XmlElement>) {
		for element in self.elements() {
			if element.local_name() == local {
				out.push(element);
			}
			element.collect_named(local, out);
		}
	}

	/// The value of the attribute written `key`, ie `r:id`.
	pub fn attribute(&self, key: &str) -> Option<&str> {
		self.attributes
			.iter()
			.find(|attribute| attribute.key == key)
			.map(|attribute| attribute.value.as_str())
	}

	/// The value of the attribute whose local name is `local`, whatever its
	/// prefix, ie `val` for `w:val`.
	pub fn attribute_local(&self, local: &str) -> Option<&str> {
		self.attributes
			.iter()
			.find(|attribute| Self::local(&attribute.key) == local)
			.map(|attribute| attribute.value.as_str())
	}

	/// The value of the prefixed attribute whose local name is `local`, ie
	/// `r:id`, never an unprefixed one, ie a slide's own `id`.
	pub fn prefixed_attribute(&self, local: &str) -> Option<&str> {
		self.attributes
			.iter()
			.find(|attribute| {
				attribute
					.key
					.split_once(':')
					.is_some_and(|(_, name)| name == local)
			})
			.map(|attribute| attribute.value.as_str())
	}

	/// The text of every descendant, joined, as written.
	pub fn text(&self) -> String {
		let mut out = String::new();
		self.collect_text(&mut out);
		out
	}

	fn collect_text(&self, out: &mut String) {
		for node in &self.children {
			match node {
				XmlNode::Element(element) => element.collect_text(out),
				XmlNode::Text(text) => out.push_str(text),
			}
		}
	}

	/// The trimmed text of the element at `path` (see [`at`](Self::at)), empty
	/// for an empty element, ie a field's value in a protocol answer.
	pub fn text_at(&self, path: &str) -> Option<&str> {
		self.at(path).map(|element| {
			element
				.children
				.iter()
				.find_map(|node| match node {
					XmlNode::Text(text) => Some(text.trim()),
					XmlNode::Element(_) => None,
				})
				.unwrap_or_default()
		})
	}

	/// `text` escaped for xml character data.
	pub fn escape_text(text: &str) -> String {
		let mut out = String::with_capacity(text.len());
		for char in text.chars() {
			match char {
				'&' => out.push_str("&amp;"),
				'<' => out.push_str("&lt;"),
				'>' => out.push_str("&gt;"),
				char => out.push(char),
			}
		}
		out
	}

	/// The local part of `name`, ie `val` for `w:val`.
	fn local(name: &str) -> &str {
		name.split_once(':').map_or(name, |(_, local)| local)
	}
}

/// One read of an xml document, at `offset` of it.
struct XmlReader<'a> {
	xml: &'a str,
	offset: usize,
}

impl<'a> XmlReader<'a> {
	/// Every top level node, the open elements a stack the next node lands in.
	fn document(mut self) -> Result<Vec<XmlNode>> {
		let mut roots = Vec::new();
		let mut open = Vec::<XmlElement>::new();
		while let Some(rest) = self.rest() {
			if rest.starts_with("<!--") {
				self.skip_past("-->", "a comment")?;
			} else if rest.starts_with("<![CDATA[") {
				self.offset += "<![CDATA[".len();
				let text = self.take_until("]]>", "a CDATA section")?;
				Self::push_text(&mut roots, &mut open, text.to_string());
			} else if rest.starts_with("<?") {
				self.skip_past("?>", "a processing instruction")?;
			} else if rest.starts_with("<!") {
				self.skip_declaration()?;
			} else if rest.starts_with("</") {
				let start = self.offset;
				self.offset += 2;
				let tag = self.take_until(">", "a closing tag")?.trim();
				match open.pop() {
					Some(element) if element.tag == tag => {
						Self::push(&mut roots, &mut open, element)
					}
					Some(element) => {
						self.offset = start;
						return Err(self.error(&format!(
							"`</{tag}>` closes `<{}>`",
							element.tag
						)));
					}
					None => {
						self.offset = start;
						return Err(self
							.error(&format!("`</{tag}>` closes no element")));
					}
				}
			} else if rest.starts_with('<') {
				let (element, closed) = self.start_tag()?;
				match closed {
					true => Self::push(&mut roots, &mut open, element),
					false => open.push(element),
				}
			} else {
				let end = rest.find('<').unwrap_or(rest.len());
				let text = html_ext::decode_references(&rest[..end]);
				self.offset += end;
				Self::push_text(&mut roots, &mut open, text);
			}
		}
		match open.pop() {
			Some(element) => {
				Err(self.error(&format!("`<{}>` is never closed", element.tag)))
			}
			None => roots.xok(),
		}
	}

	/// A start tag, its attributes, and whether it closes itself.
	fn start_tag(&mut self) -> Result<(XmlElement, bool)> {
		self.offset += 1;
		let mut element = XmlElement {
			tag: self.name().into(),
			..default()
		};
		if element.tag.is_empty() {
			return Err(self.error("a tag has no name"));
		}
		loop {
			self.skip_whitespace();
			let rest = self.rest().unwrap_or_default();
			if rest.starts_with("/>") {
				self.offset += 2;
				return (element, true).xok();
			} else if rest.starts_with('>') {
				self.offset += 1;
				return (element, false).xok();
			} else if rest.is_empty() {
				return Err(self.error(&format!(
					"the tag `<{}` is never closed",
					element.tag
				)));
			}
			let key = self.name();
			if key.is_empty() {
				return Err(self.error(&format!(
					"unexpected `{}` in `<{}>`",
					rest.chars().next().unwrap_or_default(),
					element.tag
				)));
			}
			self.skip_whitespace();
			if !self.rest().unwrap_or_default().starts_with('=') {
				return Err(
					self.error(&format!("the attribute `{key}` has no value"))
				);
			}
			self.offset += 1;
			self.skip_whitespace();
			let quote = match self.rest().and_then(|rest| rest.chars().next()) {
				Some(quote @ ('"' | '\'')) => quote,
				_ => {
					return Err(self
						.error(&format!("the attribute `{key}` is unquoted")));
				}
			};
			self.offset += 1;
			let written =
				self.take_until(&quote.to_string(), "an attribute value")?;
			element.attributes.push(XmlAttribute {
				key: key.into(),
				value: html_ext::decode_references(
					&written.replace(['\t', '\n', '\r'], " "),
				),
			});
		}
	}

	/// The unread remainder, `None` once it is all read.
	fn rest(&self) -> Option<&'a str> {
		let xml = self.xml;
		Some(&xml[self.offset..]).filter(|rest| !rest.is_empty())
	}

	/// The name at the offset, ending at whitespace, `/`, `>` or `=`.
	fn name(&mut self) -> &'a str {
		let xml = self.xml;
		let rest = &xml[self.offset..];
		let end = rest
			.find(|char: char| {
				char.is_whitespace() || matches!(char, '/' | '>' | '=')
			})
			.unwrap_or(rest.len());
		self.offset += end;
		&rest[..end]
	}

	fn skip_whitespace(&mut self) {
		let rest = &self.xml[self.offset..];
		self.offset += rest.len() - rest.trim_start().len();
	}

	/// The text up to `end`, the offset moved past it.
	fn take_until(&mut self, end: &str, what: &str) -> Result<&'a str> {
		let xml = self.xml;
		let rest = &xml[self.offset..];
		match rest.find(end) {
			Some(found) => {
				self.offset += found + end.len();
				Ok(&rest[..found])
			}
			None => Err(self.error(&format!("{what} is never closed"))),
		}
	}

	fn skip_past(&mut self, end: &str, what: &str) -> Result {
		self.take_until(end, what).map(|_| ())
	}

	/// Skips a `<!DOCTYPE ..>` or another declaration, its internal subset
	/// and quoted literals included.
	fn skip_declaration(&mut self) -> Result {
		let mut depth = 0usize;
		let mut quote = None;
		for (index, char) in self.xml[self.offset..].char_indices() {
			match (quote, char) {
				(Some(open), char) if char == open => quote = None,
				(Some(_), _) => {}
				(None, '"' | '\'') => quote = Some(char),
				(None, '[') => depth += 1,
				(None, ']') => depth = depth.saturating_sub(1),
				(None, '>') if depth == 0 => {
					self.offset += index + 1;
					return Ok(());
				}
				_ => {}
			}
		}
		Err(self.error("a declaration is never closed"))
	}

	/// Add `element` to the open element it is inside, else the document.
	fn push(
		roots: &mut Vec<XmlNode>,
		open: &mut [XmlElement],
		element: XmlElement,
	) {
		Self::nodes(roots, open).push(XmlNode::Element(element));
	}

	/// Add `text` to the open element it is inside, joining the text before
	/// it.
	fn push_text(
		roots: &mut Vec<XmlNode>,
		open: &mut [XmlElement],
		text: String,
	) {
		if text.is_empty() {
			return;
		}
		let nodes = Self::nodes(roots, open);
		match nodes.last_mut() {
			Some(XmlNode::Text(last)) => last.push_str(&text),
			_ => nodes.push(XmlNode::Text(text)),
		}
	}

	/// The nodes the next one lands in: the innermost open element's, else
	/// the document's.
	fn nodes<'n>(
		roots: &'n mut Vec<XmlNode>,
		open: &'n mut [XmlElement],
	) -> &'n mut Vec<XmlNode> {
		match open.last_mut() {
			Some(parent) => &mut parent.children,
			None => roots,
		}
	}

	/// `message` at the offset's `line:col`.
	fn error(&self, message: &str) -> BevyError {
		let before = &self.xml[..self.offset];
		let line = before.matches('\n').count() + 1;
		let col = before.len() - before.rfind('\n').map_or(0, |at| at + 1) + 1;
		bevyhow!("{message} at {line}:{col}")
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;

	/// Text is unescaped, prefixes dropped from names, and paths reach nested
	/// fields, an undeclared prefix read all the same.
	#[crate::test]
	fn reads_a_document() {
		let xml = XmlElement::parse(
			r#"<?xml version="1.0" encoding="UTF-8"?>
			<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
				<Name>bucket</Name>
				<Contents><Key>a &amp; b</Key><Size>3</Size></Contents>
				<Contents><Key>c&#47;d</Key><Size>4</Size></Contents>
				<s3:IsTruncated/>
				<Note><![CDATA[x < y]]></Note>
			</ListBucketResult>"#,
		)
		.unwrap();
		xml.local_name().xpect_eq("ListBucketResult");
		xml.text_at("Name").xpect_eq(Some("bucket"));
		xml.children_named("Contents")
			.map(|contents| contents.text_at("Key").unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["a & b".to_string(), "c/d".to_string()]);
		xml.text_at("IsTruncated").xpect_eq(Some(""));
		xml.text_at("Note").xpect_eq(Some("x < y"));
		xml.find("Size").unwrap().text().xpect_eq("3");
		XmlElement::escape_text("<a & b>").xpect_eq("&lt;a &amp; b&gt;");
	}

	/// Mixed content keeps its order and its whitespace, an attribute's
	/// literal line break the space xml makes it, and a comment, processing
	/// instruction and doctype read as nothing.
	#[crate::test]
	fn keeps_mixed_content() {
		let nodes = XmlNode::parse_document(
			"<!DOCTYPE d [<!ENTITY e \"x>\">]>\n<w:p a='1\n2' w:val=\"v\"> one <w:r>two</w:r><!-- no --><?pi x?> three </w:p>",
		)
		.unwrap();
		let root = XmlNode::document_element(&nodes).unwrap();
		root.tag.as_str().xpect_eq("w:p");
		root.attribute("a").xpect_eq(Some("1 2"));
		root.attribute_local("val").xpect_eq(Some("v"));
		root.prefixed_attribute("val").xpect_eq(Some("v"));
		root.prefixed_attribute("a").xpect_none();
		root.text().xpect_eq(" one two three ");
		root.children.len().xpect_eq(3);
	}

	#[crate::test]
	fn refuses_malformed_documents() {
		XmlElement::parse("<?xml version=\"1.0\"?>")
			.unwrap_err()
			.to_string()
			.xpect_contains("no root element");
		XmlElement::parse("<a>\n<b></a>")
			.unwrap_err()
			.to_string()
			.xpect_contains("`</a>` closes `<b>` at 2:4");
		XmlElement::parse("<a><b/>")
			.unwrap_err()
			.to_string()
			.xpect_contains("`<a>` is never closed");
	}
}
