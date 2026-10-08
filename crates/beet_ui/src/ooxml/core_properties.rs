use crate::prelude::*;
use beet_core::prelude::*;

/// A file's core properties, `docProps/core.xml`: who made it and when, read
/// into the [`PageMeta`] its tree's root carries.
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
		let nodes = BsxNode::parse_document(
			core::str::from_utf8(bytes)?,
			&BsxParseConfig::xml(),
		)?;
		let Some(root) = BsxNode::document_element(&nodes) else {
			return Self::default().xok();
		};
		let field = |local: &str| {
			root.child(local)
				.map(|element| SmolStr::from(element.text().trim()))
				.unwrap_or_default()
		};
		Self {
			title: field("title"),
			author: field("creator"),
			modified_by: field("lastModifiedBy"),
			created: field("created"),
			modified: field("modified"),
			revision: field("revision"),
		}
		.xok()
	}

	/// The day the file was created, when it names one.
	pub fn created_day(&self) -> Option<Date> { Self::day(&self.created) }

	/// The day the file was last modified, when it names one.
	pub fn modified_day(&self) -> Option<Date> { Self::day(&self.modified) }

	/// The page metadata these declare: the title, the author and the days
	/// the file was created and last modified.
	pub fn page_meta(&self) -> PageMeta {
		PageMeta {
			title: (!self.title.is_empty()).then(|| self.title.to_string()),
			authors: match self.author.is_empty() {
				true => Vec::new(),
				false => vec![self.author.clone()],
			},
			created: self.created_day(),
			updated: self.modified_day(),
			..default()
		}
	}

	/// The [`page_meta`](Self::page_meta) as the declarations a root scan
	/// reads, named as `component`.
	pub fn declarations(&self, component: &str) -> RootDeclarations {
		let text = |text: &str| DataLiteral::Scalar(Value::Str(text.into()));
		let fields = [
			(!self.title.is_empty()).then(|| ("title", text(&self.title))),
			(!self.author.is_empty()).then(|| {
				("authors", DataLiteral::List(vec![text(&self.author)]))
			}),
			self.created_day()
				.map(|day| ("created", text(&day.to_string()))),
			self.modified_day()
				.map(|day| ("updated", text(&day.to_string()))),
		]
		.into_iter()
		.flatten()
		.map(|(key, value)| (SmolStr::new(key), value))
		.collect::<Vec<_>>();
		match fields.is_empty() {
			true => RootDeclarations::default(),
			false => RootDeclarations(vec![NamedLiteral {
				name: component.into(),
				fields: NamedFields::Struct(fields),
			}]),
		}
	}

	/// The day a W3CDTF timestamp falls on, ie `2024-03-01` from
	/// `2024-03-01T09:30:00Z`.
	fn day(timestamp: &str) -> Option<Date> {
		timestamp.get(..10).and_then(|day| Date::parse(day).ok())
	}
}
