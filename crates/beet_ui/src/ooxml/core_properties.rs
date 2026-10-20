use crate::prelude::*;
use beet_core::prelude::*;

/// A file's core properties, `docProps/core.xml`, every one as written: on an
/// Office file's root beside the [`PageMeta`] it shares the title, the
/// author and the days with, and written as its own `[CoreProperties]` table
/// of that root's frontmatter, an absent property left out.
#[derive(
	Debug,
	Default,
	Clone,
	PartialEq,
	Eq,
	Component,
	Reflect,
	serde::Serialize,
	serde::Deserialize,
)]
#[reflect(Component, Default, Frontmatter)]
pub struct CoreProperties {
	/// `dc:title`.
	pub title: Option<SmolStr>,
	/// `dc:subject`.
	pub subject: Option<SmolStr>,
	/// `dc:creator`, the author.
	pub creator: Option<SmolStr>,
	/// `cp:keywords`.
	pub keywords: Option<SmolStr>,
	/// `dc:description`.
	pub description: Option<SmolStr>,
	/// `cp:lastModifiedBy`.
	pub last_modified_by: Option<SmolStr>,
	/// `cp:revision`.
	pub revision: Option<SmolStr>,
	/// `cp:lastPrinted`, as written.
	pub last_printed: Option<SmolStr>,
	/// `dcterms:created`, as written.
	pub created: Option<SmolStr>,
	/// `dcterms:modified`, as written.
	pub modified: Option<SmolStr>,
	/// `cp:category`.
	pub category: Option<SmolStr>,
	/// `cp:contentStatus`.
	pub content_status: Option<SmolStr>,
	/// `dc:identifier`.
	pub identifier: Option<SmolStr>,
	/// `dc:language`.
	pub language: Option<SmolStr>,
	/// `cp:version`.
	pub version: Option<SmolStr>,
}

impl CoreProperties {
	/// Reads the core properties part, every property absent when it has
	/// none, an empty one as absent.
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
				.filter(|text| !text.is_empty())
		};
		Self {
			title: field("title"),
			subject: field("subject"),
			creator: field("creator"),
			keywords: field("keywords"),
			description: field("description"),
			last_modified_by: field("lastModifiedBy"),
			revision: field("revision"),
			last_printed: field("lastPrinted"),
			created: field("created"),
			modified: field("modified"),
			category: field("category"),
			content_status: field("contentStatus"),
			identifier: field("identifier"),
			language: field("language"),
			version: field("version"),
		}
		.xok()
	}

	/// The page metadata these share with [`PageMeta`]: the title, the
	/// author and the days the file was created and last modified.
	pub fn page_meta(&self) -> PageMeta {
		PageMeta {
			title: self.title.as_ref().map(ToString::to_string),
			authors: self.creator.iter().cloned().collect(),
			created: self.created.as_deref().and_then(Self::day),
			updated: self.modified.as_deref().and_then(Self::day),
			..default()
		}
	}

	/// The [`page_meta`](Self::page_meta) as the declarations a root scan
	/// reads, named as `component`.
	pub fn declarations(&self, component: &str) -> RootDeclarations {
		let meta = self.page_meta();
		let text = |text: &str| DataLiteral::Scalar(Value::Str(text.into()));
		let fields = [
			meta.title.as_deref().map(|title| ("title", text(title))),
			(!meta.authors.is_empty()).then(|| {
				(
					"authors",
					DataLiteral::List(
						meta.authors
							.iter()
							.map(|author| text(author))
							.collect(),
					),
				)
			}),
			meta.created.map(|day| ("created", text(&day.to_string()))),
			meta.updated.map(|day| ("updated", text(&day.to_string()))),
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
