use crate::prelude::*;
use beet_core::prelude::*;

type Ns = OoxmlNamespace;

/// A file's core properties, `docProps/core.xml`: who made it and when, as
/// a dump's header names them.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CoreProperties {
	/// `dc:title`.
	pub title: SmolStr,
	/// `dc:creator`, the author.
	pub author: SmolStr,
	/// `cp:lastModifiedBy`.
	pub modified_by: SmolStr,
	/// `dcterms:created`, as written.
	pub created: SmolStr,
	/// `dcterms:modified`, as written.
	pub modified: SmolStr,
	/// `cp:revision`.
	pub revision: SmolStr,
}

impl CoreProperties {
	/// Reads the core properties part, every field empty when it has none.
	pub fn parse(bytes: Option<&[u8]>) -> Result<Self> {
		let Some(bytes) = bytes else {
			return Self::default().xok();
		};
		let tree = XmlTree::parse(bytes)?;
		let field = |namespace: &str, local: &str| {
			tree.root
				.child(namespace, local)
				.map(XmlElement::text)
				.map(SmolStr::from)
				.unwrap_or_default()
		};
		Self {
			title: field(Ns::DUBLIN_CORE, "title"),
			author: field(Ns::DUBLIN_CORE, "creator"),
			modified_by: field(Ns::CORE_PROPERTIES, "lastModifiedBy"),
			created: field(Ns::DUBLIN_CORE_TERMS, "created"),
			modified: field(Ns::DUBLIN_CORE_TERMS, "modified"),
			revision: field(Ns::CORE_PROPERTIES, "revision"),
		}
		.xok()
	}

	/// One sentence naming every property, empty ones as `''`.
	pub fn sentence(&self) -> String {
		format!(
			"title '{}', author '{}', last modified by '{}', created {}, \
			 modified {}, revision {}.",
			self.title,
			self.author,
			self.modified_by,
			Self::or_none(&self.created),
			Self::or_none(&self.modified),
			Self::or_none(&self.revision),
		)
	}

	/// The day the file was created, when it names one.
	pub fn created_day(&self) -> Option<Date> { Self::day(&self.created) }

	/// The day the file was last modified, when it names one.
	pub fn modified_day(&self) -> Option<Date> { Self::day(&self.modified) }

	/// The day a W3CDTF timestamp falls on, ie `2024-03-01` from
	/// `2024-03-01T09:30:00Z`.
	fn day(timestamp: &str) -> Option<Date> {
		timestamp.get(..10).and_then(|day| Date::parse(day).ok())
	}

	fn or_none(text: &str) -> &str {
		match text.is_empty() {
			true => "none",
			false => text,
		}
	}
}
