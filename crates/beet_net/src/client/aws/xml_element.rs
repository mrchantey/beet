use beet_core::prelude::*;

/// An xml document read as a plain element tree: each element's local name,
/// its text and its child elements, with attributes, namespace prefixes,
/// comments and processing instructions dropped, a prefix the document never
/// declares refused as malformed. Read with
/// [`child`](Self::child) and [`text_at`](Self::text_at) rather than a
/// schema, so a field a service adds later is simply not looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmlElement {
	/// The local name, namespace prefix dropped.
	pub name: SmolStr,
	/// The element's own text and CDATA, unescaped and trimmed.
	pub text: String,
	/// The child elements, in document order.
	pub children: Vec<XmlElement>,
}

impl XmlElement {
	/// The root element of `xml`.
	///
	/// ## Errors
	/// Errors when `xml` is not well formed or has no root element.
	pub fn parse(xml: &str) -> Result<Self> {
		let document =
			roxmltree::Document::parse(xml).map_err(|err| match err {
				roxmltree::Error::NoRootNode => {
					bevyhow!("the xml has no root element")
				}
				err => bevyhow!("malformed xml: {err}"),
			})?;
		Self::from_node(document.root_element()).xok()
	}

	fn from_node(node: roxmltree::Node) -> Self {
		let mut text = String::new();
		let mut children = Vec::new();
		for child in node.children() {
			if child.is_element() {
				children.push(Self::from_node(child));
			} else if let Some(content) =
				child.is_text().then(|| child.text()).flatten()
			{
				text.push_str(content);
			}
		}
		Self {
			name: node.tag_name().name().into(),
			text: text.trim().to_string(),
			children,
		}
	}

	/// The first child named `name`.
	pub fn child(&self, name: &str) -> Option<&XmlElement> {
		self.children.iter().find(|child| child.name == name)
	}

	/// Every child named `name`.
	pub fn children_named<'a>(
		&'a self,
		name: &'a str,
	) -> impl Iterator<Item = &'a XmlElement> {
		self.children.iter().filter(move |child| child.name == name)
	}

	/// The element at the `/` separated `path` below this one.
	pub fn at(&self, path: &str) -> Option<&XmlElement> {
		path.split('/')
			.filter(|name| !name.is_empty())
			.try_fold(self, |element, name| element.child(name))
	}

	/// The text of the element at `path`, see [`at`](Self::at).
	pub fn text_at(&self, path: &str) -> Option<&str> {
		self.at(path).map(|element| element.text.as_str())
	}

	/// The first element named `name` anywhere below this one, depth first.
	pub fn find(&self, name: &str) -> Option<&XmlElement> {
		self.children
			.iter()
			.find_map(|child| match child.name == name {
				true => Some(child),
				false => child.find(name),
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
}

#[cfg(test)]
mod test {
	use super::*;

	/// Text is unescaped, namespace prefixes dropped, and paths reach nested
	/// fields.
	#[beet_core::test]
	fn reads_a_document() {
		let xml = XmlElement::parse(
			r#"<?xml version="1.0" encoding="UTF-8"?>
			<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/" xmlns:s3="http://s3.amazonaws.com/doc/2006-03-01/">
				<Name>bucket</Name>
				<Contents><Key>a &amp; b</Key><Size>3</Size></Contents>
				<Contents><Key>c&#47;d</Key><Size>4</Size></Contents>
				<s3:IsTruncated/>
				<Note><![CDATA[x < y]]></Note>
			</ListBucketResult>"#,
		)
		.unwrap();
		xml.name.as_str().xpect_eq("ListBucketResult");
		xml.text_at("Name").xpect_eq(Some("bucket"));
		xml.children_named("Contents")
			.map(|contents| contents.text_at("Key").unwrap().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["a & b".to_string(), "c/d".to_string()]);
		xml.text_at("IsTruncated").xpect_eq(Some(""));
		xml.text_at("Note").xpect_eq(Some("x < y"));
		xml.find("Size").unwrap().text.as_str().xpect_eq("3");
		XmlElement::escape_text("<a & b>").xpect_eq("&lt;a &amp; b&gt;");
	}

	#[beet_core::test]
	fn refuses_a_document_without_a_root() {
		XmlElement::parse("<?xml version=\"1.0\"?>")
			.unwrap_err()
			.to_string()
			.xpect_contains("no root element");
	}
}
