//! Source identity: what a node was in the file it was read from, kept beside
//! what it means, so a writer can write the file back.
//!
//! A document read from a structured file is one tree whatever the format: a
//! node that means what an HTML element means carries that [`Element`], every
//! other node is still an entity with none, and the file's own vocabulary
//! rides as components on the same entities. A node with a
//! [`SourceElement`] is written from it; an entity with no source identity
//! is projection only, ie a list's `<ul>`, and a writer passes through it,
//! writing its children in place.
use crate::prelude::*;

/// The source identity of an element read from an XML part: its qualified
/// name as written, the namespace its prefix resolved to, and its attributes
/// in written order. A writer writes the element from these, never from its
/// [`Element`], which says what it means to a reader.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub struct SourceElement {
	/// The qualified name as written, ie `w:p`.
	pub name: SmolStr,
	/// The namespace the name's prefix resolved to, absent when unbound.
	pub namespace: Option<SmolStr>,
	/// The attributes in written order, namespace declarations included.
	pub attributes: Vec<SourceAttribute>,
}

/// One attribute of a [`SourceElement`]: its name as written, the namespace a
/// prefixed name resolved to, and its value with references decoded.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct SourceAttribute {
	/// The qualified name as written, ie `w:val`.
	pub name: SmolStr,
	/// The namespace a prefix resolved to; an unprefixed attribute has none.
	pub namespace: Option<SmolStr>,
	/// The value, references decoded.
	pub value: SmolStr,
}

impl SourceElement {
	/// The `xml:` prefix's namespace, bound in every document.
	pub const XML_NAMESPACE: &str = "http://www.w3.org/XML/1998/namespace";

	/// An element named `name` as written, in `namespace`, with no
	/// attributes.
	pub fn new(name: impl Into<SmolStr>, namespace: Option<SmolStr>) -> Self {
		Self {
			name: name.into(),
			namespace,
			attributes: Vec::new(),
		}
	}

	/// The name without its prefix, ie `p` for `w:p`.
	pub fn local_name(&self) -> &str { local_name(&self.name) }

	/// The prefix the name was written with, empty when unprefixed.
	pub fn prefix(&self) -> &str {
		self.name.split_once(':').map_or("", |(prefix, _)| prefix)
	}

	/// Whether this is `local` in `namespace`.
	pub fn is(&self, namespace: &str, local: &str) -> bool {
		self.namespace.as_deref() == Some(namespace)
			&& self.local_name() == local
	}

	/// The value of the attribute named `local`, in `namespace`, or
	/// unprefixed when `None`.
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

	/// Sets `attribute`, replacing one of the same name and namespace, else
	/// appending it.
	pub fn set_attribute(&mut self, attribute: SourceAttribute) {
		let local = attribute.local_name().to_owned();
		match self.attributes.iter_mut().find(|existing| {
			existing.matches(attribute.namespace.as_deref(), &local)
		}) {
			Some(existing) => existing.value = attribute.value,
			None => self.attributes.push(attribute),
		}
	}

	/// Removes the attribute named `local` in `namespace`, answering whether
	/// there was one.
	pub fn remove_attribute(
		&mut self,
		namespace: Option<&str>,
		local: &str,
	) -> bool {
		let before = self.attributes.len();
		self.attributes
			.retain(|attribute| !attribute.matches(namespace, local));
		self.attributes.len() != before
	}

	/// A sibling element in `namespace` named `local`, written with this
	/// element's prefix when they share its namespace, ie a new `w:r` beside
	/// a `w:p`.
	pub fn sibling(&self, namespace: &str, local: &str) -> Self {
		let name = match (self.namespace.as_deref(), self.prefix()) {
			(Some(own), prefix) if own == namespace && !prefix.is_empty() => {
				format!("{prefix}:{local}")
			}
			_ => local.to_owned(),
		};
		Self::new(name, Some(namespace.into()))
	}
}

impl SourceAttribute {
	/// An attribute named `name` as written, in `namespace`.
	pub fn new(
		name: impl Into<SmolStr>,
		namespace: Option<SmolStr>,
		value: impl Into<SmolStr>,
	) -> Self {
		Self {
			name: name.into(),
			namespace,
			value: value.into(),
		}
	}

	/// The name without its prefix.
	pub fn local_name(&self) -> &str { local_name(&self.name) }

	fn matches(&self, namespace: Option<&str>, local: &str) -> bool {
		self.namespace.as_deref() == namespace && self.local_name() == local
	}
}

/// A qualified name without its prefix.
fn local_name(name: &str) -> &str {
	name.split_once(':').map_or(name, |(_, local)| local)
}

/// Text a writer writes and no reader sees, ie a field's instruction or a
/// checkbox's glyph, which its `<input>` already shows: kept here rather than
/// in a [`Value`], which every renderer reads.
#[derive(Debug, Clone, PartialEq, Eq, Deref, DerefMut, Reflect, Component)]
#[reflect(Component)]
pub struct SourceText(pub String);

impl SourceText {
	/// Text written but never read.
	pub fn new(text: impl Into<String>) -> Self { Self(text.into()) }
}

/// A node's position among its source siblings, recorded on a node a
/// projection moved, ie a slide's shape put in reading order, so a writer
/// restores the order the file had.
#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Deref,
	Reflect,
	Component,
)]
#[reflect(Component)]
pub struct SourceOrder(pub u32);

/// The root of one part of a source package, ie `word/document.xml`: its
/// children are the part's prolog and its document element. A writer writes
/// one part at a time, so a part nested in another's subtree, ie a diagram's
/// data beside the frame showing it, is never written into its host.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub struct SourcePart {
	/// The part's path in its package, ie `word/document.xml`.
	pub path: SmolStr,
}

impl SourcePart {
	/// The part at `path`.
	pub fn new(path: impl Into<SmolStr>) -> Self { Self { path: path.into() } }
}
