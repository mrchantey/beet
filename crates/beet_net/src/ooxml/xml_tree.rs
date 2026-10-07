use beet_core::prelude::*;
use quick_xml::events::BytesStart;
use quick_xml::events::Event;

/// One part's XML as an element tree: what precedes the root as written, and
/// the root. A Word or slide part is edited through it because its operations
/// are defined over descendants in document order, which a typed schema walk
/// would enumerate container by container.
///
/// Names keep their prefix as written and carry the namespace it resolved to,
/// so a query matches by namespace while the written form round-trips; text
/// and attribute values are held unescaped and escaped again on write.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlTree {
	/// The declaration, comments and processing instructions before the root.
	prolog: Vec<XmlNode>,
	/// The document element.
	pub root: XmlElement,
}

/// A child of an element.
#[derive(Debug, Clone, PartialEq)]
pub enum XmlNode {
	/// An element.
	Element(XmlElement),
	/// Character data, unescaped.
	Text(String),
	/// A `<![CDATA[..]]>` section's content.
	CData(String),
	/// A comment's content.
	Comment(String),
	/// A declaration, processing instruction or doctype, as written.
	Raw(String),
}

/// An element: its name as written, the namespace that name resolved to, its
/// attributes in order and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlElement {
	/// The qualified name as written, ie `w:p`.
	pub name: SmolStr,
	/// The namespace the prefix resolved to, absent when unbound.
	pub namespace: Option<SmolStr>,
	/// The attributes in written order, namespace declarations included.
	pub attributes: Vec<XmlAttribute>,
	/// The children in order.
	pub children: Vec<XmlNode>,
}

/// An attribute: its name as written, the namespace a prefixed name resolved
/// to, and its unescaped value.
#[derive(Debug, Clone, PartialEq)]
pub struct XmlAttribute {
	/// The qualified name as written, ie `w:val`.
	pub name: SmolStr,
	/// The namespace a prefix resolved to; an unprefixed attribute has none.
	pub namespace: Option<SmolStr>,
	/// The value, unescaped.
	pub value: String,
}

impl XmlTree {
	/// The `xml:` prefix's namespace, bound in every document.
	pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

	/// Parses a part's bytes, a leading byte order mark dropped.
	pub fn parse(bytes: &[u8]) -> Result<Self> {
		let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
		let mut reader = quick_xml::Reader::from_reader(bytes);
		reader.config_mut().trim_text(false);
		let mut prolog = Vec::new();
		let mut scopes = NamespaceScopes::default();
		// the open elements, innermost last
		let mut stack: Vec<XmlElement> = Vec::new();
		let mut root = None;
		loop {
			let event = reader.read_event()?;
			match event {
				Event::Start(start) => {
					stack.push(scopes.open(&start)?);
				}
				Event::Empty(start) => {
					let element = scopes.open(&start)?;
					scopes.close();
					Self::attach(&mut stack, &mut root, element)?;
				}
				Event::End(_) => {
					let element = stack.pop().ok_or_else(|| {
						bevyhow!("a closing tag closes nothing")
					})?;
					scopes.close();
					Self::attach(&mut stack, &mut root, element)?;
				}
				Event::Text(text) => {
					Self::push_text(&mut stack, &text.decode()?);
				}
				Event::GeneralRef(reference) => {
					let text = Self::resolve_reference(&reference.decode()?)?;
					Self::push_text(&mut stack, &text);
				}
				Event::CData(data) => {
					let text = data.decode()?.into_owned();
					Self::push_node(
						&mut stack,
						&mut prolog,
						XmlNode::CData(text),
					);
				}
				Event::Comment(comment) => {
					let text = comment.decode()?.into_owned();
					Self::push_node(
						&mut stack,
						&mut prolog,
						XmlNode::Comment(text),
					);
				}
				Event::Decl(decl) => {
					let text = String::from_utf8_lossy(&decl).into_owned();
					Self::push_node(
						&mut stack,
						&mut prolog,
						XmlNode::Raw(format!("<?{text}?>")),
					);
				}
				Event::PI(pi) => {
					let text = String::from_utf8_lossy(&pi).into_owned();
					Self::push_node(
						&mut stack,
						&mut prolog,
						XmlNode::Raw(format!("<?{text}?>")),
					);
				}
				Event::DocType(doctype) => {
					let text = doctype.decode()?.into_owned();
					Self::push_node(
						&mut stack,
						&mut prolog,
						XmlNode::Raw(format!("<!DOCTYPE {text}>")),
					);
				}
				Event::Eof => break,
			}
		}
		match (root, stack.is_empty()) {
			(Some(root), true) => Self { prolog, root }.xok(),
			(_, false) => bevybail!("the xml ends inside an open element"),
			(None, true) => bevybail!("the xml has no root element"),
		}
	}

	/// The tree as bytes, the prolog as written and the rest escaped.
	pub fn to_bytes(&self) -> Vec<u8> {
		let mut out = String::new();
		for node in &self.prolog {
			node.write(&mut out);
			// a declaration ends its line, as every Office part writes it
			if matches!(node, XmlNode::Raw(_)) {
				out.push_str("\r\n");
			}
		}
		self.root.write(&mut out);
		out.into_bytes()
	}

	/// The prefix the root binds to `namespace`, ie `w` for WordprocessingML.
	pub fn prefix_of(&self, namespace: &str) -> Option<&str> {
		self.root.attributes.iter().find_map(|attribute| {
			(attribute.value == namespace)
				.then(|| attribute.name.strip_prefix("xmlns:"))
				.flatten()
		})
	}

	/// A new empty element in `namespace`, named with the prefix the root
	/// binds to it.
	pub fn element(&self, namespace: &str, local: &str) -> Result<XmlElement> {
		let prefix = self.prefix_of(namespace).ok_or_else(|| {
			bevyhow!("the part binds no prefix to the namespace `{namespace}`")
		})?;
		XmlElement::new(format!("{prefix}:{local}"), Some(namespace)).xok()
	}

	/// A new attribute in `namespace`, named with the prefix the root binds
	/// to it, or `xml:` for the xml namespace.
	pub fn attribute(
		&self,
		namespace: &str,
		local: &str,
		value: impl Into<String>,
	) -> Result<XmlAttribute> {
		let prefix = match namespace {
			Self::XML_NAMESPACE => "xml",
			_ => self.prefix_of(namespace).ok_or_else(|| {
				bevyhow!(
					"the part binds no prefix to the namespace `{namespace}`"
				)
			})?,
		};
		XmlAttribute {
			name: format!("{prefix}:{local}").into(),
			namespace: Some(namespace.into()),
			value: value.into(),
		}
		.xok()
	}

	/// Hands a closed element to its parent, or makes it the root.
	fn attach(
		stack: &mut Vec<XmlElement>,
		root: &mut Option<XmlElement>,
		element: XmlElement,
	) -> Result {
		match stack.last_mut() {
			Some(parent) => parent.children.push(XmlNode::Element(element)),
			None if root.is_none() => *root = Some(element),
			None => bevybail!("the xml has more than one root element"),
		}
		Ok(())
	}

	/// Text joins the text before it, so a reference splits nothing; text
	/// outside the root, whitespace between prolog nodes, is the writer's to
	/// choose.
	fn push_text(stack: &mut Vec<XmlElement>, text: &str) {
		if let Some(parent) = stack.last_mut() {
			match parent.children.last_mut() {
				Some(XmlNode::Text(existing)) => existing.push_str(text),
				_ => parent.children.push(XmlNode::Text(text.to_string())),
			}
		}
	}

	fn push_node(
		stack: &mut Vec<XmlElement>,
		prolog: &mut Vec<XmlNode>,
		node: XmlNode,
	) {
		match stack.last_mut() {
			Some(parent) => parent.children.push(node),
			None => prolog.push(node),
		}
	}

	/// The character a `&name;` reference stands for.
	fn resolve_reference(name: &str) -> Result<String> {
		let char = match name {
			"amp" => '&',
			"lt" => '<',
			"gt" => '>',
			"quot" => '"',
			"apos" => '\'',
			other => {
				let code = match other.strip_prefix("#x") {
					Some(hex) => u32::from_str_radix(hex, 16).ok(),
					None => other
						.strip_prefix('#')
						.and_then(|decimal| decimal.parse().ok()),
				};
				code.and_then(char::from_u32).ok_or_else(|| {
					bevyhow!("`&{other};` is not an xml reference")
				})?
			}
		};
		char.to_string().xok()
	}
}

impl XmlNode {
	fn write(&self, out: &mut String) {
		match self {
			Self::Element(element) => element.write(out),
			Self::Text(text) => {
				out.push_str(&quick_xml::escape::partial_escape(text))
			}
			Self::CData(text) => {
				out.push_str("<![CDATA[");
				out.push_str(text);
				out.push_str("]]>");
			}
			Self::Comment(text) => {
				out.push_str("<!--");
				out.push_str(text);
				out.push_str("-->");
			}
			Self::Raw(text) => out.push_str(text),
		}
	}
}

impl XmlElement {
	/// An empty element named `name` as written, in `namespace`.
	pub fn new(name: impl Into<SmolStr>, namespace: Option<&str>) -> Self {
		Self {
			name: name.into(),
			namespace: namespace.map(SmolStr::new),
			attributes: Vec::new(),
			children: Vec::new(),
		}
	}

	/// The name without its prefix, ie `p` for `w:p`.
	pub fn local_name(&self) -> &str {
		self.name
			.split_once(':')
			.map_or(self.name.as_str(), |(_, local)| local)
	}

	/// Whether this is `local` in `namespace`.
	pub fn is(&self, namespace: &str, local: &str) -> bool {
		self.namespace.as_deref() == Some(namespace)
			&& self.local_name() == local
	}

	/// The element children, in order.
	pub fn elements(&self) -> impl Iterator<Item = &XmlElement> {
		self.children.iter().filter_map(|node| match node {
			XmlNode::Element(element) => Some(element),
			_ => None,
		})
	}

	/// The element children named `local` in `namespace`, in order.
	pub fn children_named<'a, 'name>(
		&'a self,
		namespace: &'name str,
		local: &'name str,
	) -> impl Iterator<Item = &'a XmlElement> {
		self.elements()
			.filter(move |element| element.is(namespace, local))
	}

	/// The first element child named `local` in `namespace`.
	pub fn child(&self, namespace: &str, local: &str) -> Option<&XmlElement> {
		self.children_named(namespace, local).next()
	}

	/// The first element child named `local` in `namespace`, mutably.
	pub fn child_mut(
		&mut self,
		namespace: &str,
		local: &str,
	) -> Option<&mut XmlElement> {
		self.children.iter_mut().find_map(|node| match node {
			XmlNode::Element(element) if element.is(namespace, local) => {
				Some(element)
			}
			_ => None,
		})
	}

	/// Every descendant element in document order, this one excluded.
	pub fn descendants(&self) -> Vec<&XmlElement> {
		let mut out = Vec::new();
		self.collect_descendants(&mut out);
		out
	}

	fn collect_descendants<'a>(&'a self, out: &mut Vec<&'a XmlElement>) {
		for element in self.elements() {
			out.push(element);
			element.collect_descendants(out);
		}
	}

	/// Every descendant named `local` in `namespace`, in document order.
	pub fn descendants_named(
		&self,
		namespace: &str,
		local: &str,
	) -> Vec<&XmlElement> {
		self.descendants()
			.into_iter()
			.filter(|element| element.is(namespace, local))
			.collect()
	}

	/// The text of every descendant text node, joined.
	pub fn text(&self) -> String {
		let mut out = String::new();
		self.collect_text(&mut out);
		out
	}

	fn collect_text(&self, out: &mut String) {
		for node in &self.children {
			match node {
				XmlNode::Element(element) => element.collect_text(out),
				XmlNode::Text(text) | XmlNode::CData(text) => {
					out.push_str(text)
				}
				_ => {}
			}
		}
	}

	/// The text of every descendant named `local` in `namespace`, joined, ie
	/// a Word paragraph's `w:t` runs.
	pub fn text_of(&self, namespace: &str, local: &str) -> String {
		self.descendants_named(namespace, local)
			.into_iter()
			.map(XmlElement::text)
			.collect()
	}

	/// Replaces the children with one text node.
	pub fn set_text(&mut self, text: impl Into<String>) {
		self.children = vec![XmlNode::Text(text.into())];
	}

	/// The value of the attribute named `local`, in `namespace` or unprefixed
	/// when `None`.
	pub fn attribute(
		&self,
		namespace: Option<&str>,
		local: &str,
	) -> Option<&str> {
		self.attributes
			.iter()
			.find(|attribute| attribute.matches(namespace, local))
			.map(|attribute| attribute.value.as_str())
	}

	/// Sets `attribute`, replacing one of the same name and namespace.
	pub fn set_attribute(&mut self, attribute: XmlAttribute) {
		let local = attribute.local_name().to_string();
		match self.attributes.iter_mut().find(|existing| {
			existing.matches(attribute.namespace.as_deref(), &local)
		}) {
			Some(existing) => existing.value = attribute.value,
			None => self.attributes.push(attribute),
		}
	}

	/// The child index paths of every descendant `predicate` accepts, in
	/// document order: `[2, 0]` is the first child of the third child.
	/// Removing by path goes in reverse, so an earlier path stays valid.
	pub fn paths(
		&self,
		predicate: impl Fn(&XmlElement) -> bool,
	) -> Vec<Vec<usize>> {
		let mut out = Vec::new();
		let mut path = Vec::new();
		self.collect_paths(&predicate, &mut path, &mut out);
		out
	}

	fn collect_paths(
		&self,
		predicate: &impl Fn(&XmlElement) -> bool,
		path: &mut Vec<usize>,
		out: &mut Vec<Vec<usize>>,
	) {
		for (index, node) in self.children.iter().enumerate() {
			if let XmlNode::Element(element) = node {
				path.push(index);
				if predicate(element) {
					out.push(path.clone());
				}
				element.collect_paths(predicate, path, out);
				path.pop();
			}
		}
	}

	/// The descendant at `path`, this element for an empty one.
	pub fn at(&self, path: &[usize]) -> Option<&XmlElement> {
		path.iter().try_fold(self, |element, index| {
			match element.children.get(*index)? {
				XmlNode::Element(child) => Some(child),
				_ => None,
			}
		})
	}

	/// The descendant at `path`, mutably.
	pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut XmlElement> {
		path.iter().try_fold(self, |element, index| {
			match element.children.get_mut(*index)? {
				XmlNode::Element(child) => Some(child),
				_ => None,
			}
		})
	}

	fn write(&self, out: &mut String) {
		out.push('<');
		out.push_str(&self.name);
		for attribute in &self.attributes {
			out.push(' ');
			out.push_str(&attribute.name);
			out.push_str("=\"");
			out.push_str(&XmlAttribute::encode(&attribute.value));
			out.push('"');
		}
		if self.children.is_empty() {
			out.push_str("/>");
			return;
		}
		out.push('>');
		for child in &self.children {
			child.write(out);
		}
		out.push_str("</");
		out.push_str(&self.name);
		out.push('>');
	}
}

impl XmlAttribute {
	/// A written value as the specification normalizes it: a literal tab or
	/// line break is a space, while a reference to one, `&#xA;`, is kept.
	fn decode(raw: &str) -> Result<String> {
		let spaced = raw.replace(['\t', '\n', '\r'], " ");
		quick_xml::escape::unescape(&spaced)?.into_owned().xok()
	}

	/// A value as written, quotes and markup escaped and whitespace other
	/// than a space as a reference, so it reads back unchanged.
	fn encode(value: &str) -> String {
		quick_xml::escape::escape(value)
			.replace('\t', "&#x9;")
			.replace('\n', "&#xA;")
			.replace('\r', "&#xD;")
	}

	/// The name without its prefix.
	pub fn local_name(&self) -> &str {
		self.name
			.split_once(':')
			.map_or(self.name.as_str(), |(_, local)| local)
	}

	fn matches(&self, namespace: Option<&str>, local: &str) -> bool {
		self.namespace.as_deref() == namespace && self.local_name() == local
	}
}

/// The prefix bindings in force at each open element, innermost last.
#[derive(Default)]
struct NamespaceScopes {
	scopes: Vec<Vec<(SmolStr, SmolStr)>>,
}

impl NamespaceScopes {
	/// Opens `start`'s scope and resolves its name and attributes in it.
	fn open(&mut self, start: &BytesStart) -> Result<XmlElement> {
		let name = SmolStr::new(str::from_utf8(start.name().as_ref())?);
		let mut attributes = Vec::new();
		let mut bindings = Vec::new();
		for attribute in start.attributes() {
			let attribute = attribute?;
			let key = str::from_utf8(attribute.key.as_ref())?.to_string();
			let value =
				XmlAttribute::decode(str::from_utf8(&attribute.value)?)?;
			if key == "xmlns" {
				bindings.push((SmolStr::default(), SmolStr::new(&value)));
			} else if let Some(prefix) = key.strip_prefix("xmlns:") {
				bindings.push((SmolStr::new(prefix), SmolStr::new(&value)));
			}
			attributes.push((key, value));
		}
		self.scopes.push(bindings);
		let element_prefix =
			name.split_once(':').map_or("", |(prefix, _)| prefix);
		XmlElement {
			namespace: self.resolve(element_prefix),
			name,
			attributes: attributes
				.into_iter()
				.map(|(key, value)| XmlAttribute {
					// an unprefixed attribute is in no namespace
					namespace: key
						.split_once(':')
						.filter(|(prefix, _)| *prefix != "xmlns")
						.and_then(|(prefix, _)| self.resolve(prefix)),
					name: key.into(),
					value,
				})
				.collect(),
			children: Vec::new(),
		}
		.xok()
	}

	fn close(&mut self) { self.scopes.pop(); }

	fn resolve(&self, prefix: &str) -> Option<SmolStr> {
		if prefix == "xml" {
			return Some(XmlTree::XML_NAMESPACE.into());
		}
		self.scopes
			.iter()
			.rev()
			.flat_map(|scope| scope.iter())
			.find(|(bound, _)| bound == prefix)
			.map(|(_, namespace)| namespace.clone())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	const NS: &str = "urn:test";

	#[beet_core::test]
	fn round_trips_and_resolves_namespaces() {
		let xml = "<?xml version=\"1.0\" standalone=\"yes\"?>\r\n<t:root xmlns:t=\"urn:test\" a=\"1 &amp; 2\"><t:p>one &lt;two&gt;</t:p><t:p/><other/></t:root>";
		let tree = XmlTree::parse(xml.as_bytes()).unwrap();
		String::from_utf8(tree.to_bytes()).unwrap().xpect_eq(xml);
		tree.root.descendants_named(NS, "p").len().xpect_eq(2);
		tree.root.text().xpect_eq("one <two>");
		tree.root.attribute(None, "a").xpect_eq(Some("1 & 2"));
		tree.prefix_of(NS).xpect_eq(Some("t"));
		tree.root.elements().last().unwrap().namespace.xpect_none();
	}

	#[beet_core::test]
	fn keeps_referenced_whitespace_in_attributes() {
		let xml = "<t:a xmlns:t=\"urn:test\" data=\"one&#xA;two\" spaced=\"one\ntwo\"/>";
		let tree = XmlTree::parse(xml.as_bytes()).unwrap();
		tree.root.attribute(None, "data").xpect_eq(Some("one\ntwo"));
		tree.root
			.attribute(None, "spaced")
			.xpect_eq(Some("one two"));
		String::from_utf8(tree.to_bytes())
			.unwrap()
			.xpect_contains("data=\"one&#xA;two\"");
	}

	#[beet_core::test]
	fn paths_index_document_order() {
		let tree = XmlTree::parse(
			b"<t:a xmlns:t=\"urn:test\"><t:p><t:p/></t:p>text<t:p/></t:a>",
		)
		.unwrap();
		let paths = tree.root.paths(|element| element.is(NS, "p"));
		paths.clone().xpect_eq(vec![vec![0], vec![0, 0], vec![2]]);
		tree.root.at(&paths[1]).unwrap().is(NS, "p").xpect_true();
	}
}
