use beet_core::prelude::*;
use quick_xml::events::Event;

/// An xml element as an AWS answer carries one: a name, its text and its
/// children, with attributes and namespaces dropped, which no AWS response
/// beet reads relies on. Read with [`child`](Self::child) and
/// [`text_at`](Self::text_at) rather than a schema, so a field AWS adds later
/// is simply not looked at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XmlElement {
	/// The local name, namespace prefix dropped.
	pub name: SmolStr,
	/// The element's own text, unescaped and trimmed.
	pub text: String,
	/// The child elements, in document order.
	pub children: Vec<XmlElement>,
}

impl XmlElement {
	/// The root element of `xml`.
	pub fn parse(xml: &str) -> Result<Self> {
		let mut reader = quick_xml::Reader::from_str(xml);
		let mut stack: Vec<XmlElement> = Vec::new();
		loop {
			match reader.read_event()? {
				Event::Start(start) => stack.push(Self {
					name: SmolStr::new(str::from_utf8(
						start.local_name().as_ref(),
					)?),
					..default()
				}),
				Event::Empty(start) => {
					let element = Self {
						name: SmolStr::new(str::from_utf8(
							start.local_name().as_ref(),
						)?),
						..default()
					};
					match stack.last_mut() {
						Some(parent) => parent.children.push(element),
						None => return Ok(element),
					}
				}
				Event::Text(text) => {
					if let Some(element) = stack.last_mut() {
						element.text.push_str(&text.decode()?);
					}
				}
				Event::CData(text) => {
					if let Some(element) = stack.last_mut() {
						element.text.push_str(&text.decode()?);
					}
				}
				Event::GeneralRef(reference) => {
					if let Some(element) = stack.last_mut() {
						match reference.resolve_char_ref()? {
							Some(char) => element.text.push(char),
							None => element.text.push_str(
								&quick_xml::escape::unescape(&format!(
									"&{};",
									reference.decode()?
								))?,
							),
						}
					}
				}
				Event::End(_) => {
					let Some(mut element) = stack.pop() else {
						bevybail!("an xml end tag closes nothing");
					};
					element.text = element.text.trim().to_string();
					match stack.last_mut() {
						Some(parent) => parent.children.push(element),
						None => return Ok(element),
					}
				}
				Event::Eof => bevybail!("the xml has no root element"),
				_ => {}
			}
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

	/// `text` escaped for an xml body.
	pub fn escape(text: &str) -> String {
		quick_xml::escape::escape(text).into_owned()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// Text is unescaped, namespaces dropped, and paths reach nested fields.
	#[beet_core::test]
	fn reads_an_answer() {
		let xml = XmlElement::parse(
			r#"<?xml version="1.0" encoding="UTF-8"?>
			<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/">
				<Name>bucket</Name>
				<Contents><Key>a &amp; b</Key><Size>3</Size></Contents>
				<Contents><Key>c&#47;d</Key><Size>4</Size></Contents>
				<IsTruncated/>
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
		xml.find("Size").unwrap().text.as_str().xpect_eq("3");
		XmlElement::escape("<a & b>").xpect_eq("&lt;a &amp; b&gt;");
	}
}
