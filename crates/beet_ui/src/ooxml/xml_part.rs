//! [`XmlPart`], one XML part of an Office package as a writer builds it.
use crate::prelude::*;
use beet_core::prelude::*;

/// An XML part being written, element by element, its text and attribute
/// values escaped as [`XmlWriter`] escapes them, led by the declaration
/// Office writes.
#[derive(Debug, Clone)]
pub(crate) struct XmlPart {
	out: String,
}

impl Default for XmlPart {
	fn default() -> Self { Self::new() }
}

impl XmlPart {
	/// A part holding only the XML declaration.
	pub fn new() -> Self {
		Self {
			out: String::from(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n",
			),
		}
	}

	/// A fragment holding nothing, no declaration, for [`raw`](Self::raw).
	pub fn fragment() -> Self { Self { out: String::new() } }

	/// Opens `name` with `attributes`, each a name and its unescaped value.
	pub fn open(
		&mut self,
		name: &str,
		attributes: &[(&str, &str)],
	) -> &mut Self {
		self.out.push('<');
		self.out.push_str(name);
		for (key, value) in attributes {
			XmlWriter::write_attribute(key, value, &mut self.out);
		}
		self.out.push('>');
		self
	}

	/// Closes `name`.
	pub fn close(&mut self, name: &str) -> &mut Self {
		self.out.push_str("</");
		self.out.push_str(name);
		self.out.push('>');
		self
	}

	/// Writes `name` with `attributes` and no content, `<name/>`.
	pub fn empty(
		&mut self,
		name: &str,
		attributes: &[(&str, &str)],
	) -> &mut Self {
		self.out.push('<');
		self.out.push_str(name);
		for (key, value) in attributes {
			XmlWriter::write_attribute(key, value, &mut self.out);
		}
		self.out.push_str("/>");
		self
	}

	/// Writes `text`, escaped.
	pub fn text(&mut self, text: &str) -> &mut Self {
		self.out.push_str(&XmlElement::escape_text(text));
		self
	}

	/// Writes `name` holding only `text`.
	pub fn text_element(
		&mut self,
		name: &str,
		attributes: &[(&str, &str)],
		text: &str,
	) -> &mut Self {
		self.open(name, attributes).text(text).close(name)
	}

	/// Writes `xml` as it is, ie a fragment another builder wrote.
	pub fn raw(&mut self, xml: &str) -> &mut Self {
		self.out.push_str(xml);
		self
	}

	/// The part's bytes.
	pub fn into_bytes(self) -> Vec<u8> { self.out.into_bytes() }

	/// The part's text.
	pub fn as_str(&self) -> &str { &self.out }
}
