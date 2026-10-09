//! [`OoxmlNode`], what an Office file's XML says at an entity.
use crate::prelude::*;
use beet_core::prelude::*;

/// What an Office file's XML says at this entity, which [`OoxmlRenderer`]
/// writes back. No renderer reads it: what a node means to a reader is the
/// [`Element`] and [`Value`] a projection puts beside it, so a node meaning
/// nothing to a reader, ie a paragraph's properties, is an entity every
/// renderer walks through. The text a reader reads is a [`Value`] on its own
/// entity, written as text where an [`OoxmlNode::Element`] holds it.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component)]
pub enum OoxmlNode {
	/// One part of the package, ie `word/document.xml`, its declaration and
	/// its root element its children. A part nested in another's subtree, ie
	/// a diagram's data beside the frame showing it, is written on its own.
	Part {
		/// The part's path in its package.
		path: SmolStr,
	},
	/// An element as written.
	Element(OoxmlElement),
	/// Character data that is no content: a field's instruction, a
	/// checkbox's glyph, which its `<input>` shows, a shared string's index,
	/// whitespace between tags.
	Text(String),
	/// A comment.
	Comment(String),
	/// A processing instruction, ie the `<?xml ..?>` declaration.
	Instruction(String),
}

impl OoxmlNode {
	/// The element, when this is one.
	pub fn element(&self) -> Option<&OoxmlElement> {
		match self {
			Self::Element(element) => Some(element),
			_ => None,
		}
	}

	/// The element to edit, when this is one.
	pub fn element_mut(&mut self) -> Option<&mut OoxmlElement> {
		match self {
			Self::Element(element) => Some(element),
			_ => None,
		}
	}

	/// Whether this is a part.
	pub fn is_part(&self) -> bool { matches!(self, Self::Part { .. }) }

	/// Spawns the part at `path` read as `nodes`: an element an
	/// [`OoxmlNode::Element`] with its name's and attributes' prefixes
	/// resolved against the declarations in scope, text a [`Value`], CDATA
	/// text too since its spelling is lexical, and a comment or an
	/// instruction its own node. No entity carries an [`Element`] yet: what
	/// a node means to a reader is the format's projection to add.
	pub(crate) fn spawn_part(
		world: &mut World,
		path: impl Into<SmolStr>,
		nodes: &[BsxNode],
	) -> Result<Entity> {
		let part = world.spawn(Self::Part { path: path.into() }).id();
		let mut scopes = NamespaceScopes::default();
		for node in nodes {
			Self::spawn_node(node, part, &mut scopes, world)?;
		}
		part.xok()
	}

	fn spawn_node(
		node: &BsxNode,
		parent: Entity,
		scopes: &mut NamespaceScopes,
		world: &mut World,
	) -> Result {
		match node {
			BsxNode::Element(element) => {
				let entity = world
					.spawn((
						ChildOf(parent),
						Self::Element(scopes.open(element)),
					))
					.id();
				for child in &element.children {
					Self::spawn_node(child, entity, scopes, world)?;
				}
				scopes.close();
			}
			BsxNode::Text(text) | BsxNode::CData(text) => {
				world.spawn((ChildOf(parent), Value::Str(text.into())));
			}
			BsxNode::Comment(text) => {
				world.spawn((ChildOf(parent), Self::Comment(text.clone())));
			}
			BsxNode::ProcessingInstruction(text) => {
				world.spawn((ChildOf(parent), Self::Instruction(text.clone())));
			}
			BsxNode::Doctype(_) => {
				bevybail!("an Office part declares no doctype")
			}
			// the XML dialect reads no expression
			BsxNode::Expr(_) => {}
		}
		Ok(())
	}
}

/// An element as an Office part writes it: its qualified name, the namespace
/// its prefix resolved to, and its attributes in written order.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct OoxmlElement {
	/// The qualified name as written, ie `w:p`.
	pub name: SmolStr,
	/// The namespace the name's prefix resolved to, absent when unbound.
	pub namespace: Option<SmolStr>,
	/// The attributes in written order, namespace declarations included.
	pub attributes: Vec<OoxmlAttribute>,
}

impl OoxmlElement {
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
	pub fn local_name(&self) -> &str { OoxmlAttribute::local(&self.name) }

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
	pub fn set_attribute(&mut self, attribute: OoxmlAttribute) {
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

/// One attribute of an [`OoxmlElement`]: its name as written, the namespace
/// a prefixed name resolved to, and its value with references decoded.
#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct OoxmlAttribute {
	/// The qualified name as written, ie `w:val`.
	pub name: SmolStr,
	/// The namespace a prefix resolved to; an unprefixed attribute has none.
	pub namespace: Option<SmolStr>,
	/// The value, references decoded.
	pub value: SmolStr,
}

impl OoxmlAttribute {
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
	pub fn local_name(&self) -> &str { Self::local(&self.name) }

	fn matches(&self, namespace: Option<&str>, local: &str) -> bool {
		self.namespace.as_deref() == namespace && self.local_name() == local
	}

	/// A qualified name without its prefix.
	fn local(name: &str) -> &str {
		name.split_once(':').map_or(name, |(_, local)| local)
	}
}

/// The prefix bindings in force at each open element, innermost last.
#[derive(Default)]
struct NamespaceScopes {
	scopes: Vec<Vec<(SmolStr, SmolStr)>>,
}

impl NamespaceScopes {
	/// Opens `element`'s scope and resolves its name and attributes in it.
	fn open(&mut self, element: &BsxElement) -> OoxmlElement {
		let attributes = element
			.attributes
			.iter()
			.map(|attribute| {
				let value = match &attribute.value {
					AttrValue::Str(value) => SmolStr::new(value),
					_ => SmolStr::default(),
				};
				(attribute.key.as_str(), value)
			})
			.collect::<Vec<_>>();
		self.scopes.push(
			attributes
				.iter()
				.filter_map(|(key, value)| match *key {
					"xmlns" => Some((SmolStr::default(), value.clone())),
					key => key
						.strip_prefix("xmlns:")
						.map(|prefix| (SmolStr::new(prefix), value.clone())),
				})
				.collect(),
		);
		let prefix =
			element.tag.split_once(':').map_or("", |(prefix, _)| prefix);
		OoxmlElement {
			name: element.tag.as_str().into(),
			namespace: self.resolve(prefix),
			attributes: attributes
				.into_iter()
				.map(|(key, value)| OoxmlAttribute {
					// an unprefixed attribute is in no namespace, and a
					// declaration declares rather than belongs to one
					namespace: key
						.split_once(':')
						.filter(|(prefix, _)| *prefix != "xmlns")
						.and_then(|(prefix, _)| self.resolve(prefix)),
					name: key.into(),
					value,
				})
				.collect(),
		}
	}

	fn close(&mut self) { self.scopes.pop(); }

	fn resolve(&self, prefix: &str) -> Option<SmolStr> {
		if prefix == "xml" {
			return Some(OoxmlNamespace::XML.into());
		}
		self.scopes
			.iter()
			.rev()
			.flat_map(|scope| scope.iter())
			.find(|(bound, _)| bound == prefix)
			.map(|(_, namespace)| namespace.clone())
			.filter(|namespace| !namespace.is_empty())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every prefix resolves against the declarations in scope, the `xml:`
	/// prefix bound everywhere, and no entity means anything to a reader yet.
	#[beet_core::test]
	fn resolves_namespaces_in_scope() {
		let nodes = BsxNode::parse_document(
			"<?xml version=\"1.0\"?>\n<t:root xmlns:t=\"urn:test\" xmlns=\"urn:default\" a=\"1 &amp; 2\"><t:p xml:space=\"preserve\"> one </t:p><Other t:b=\"x\"/><![CDATA[<raw>]]><!-- note --></t:root>",
			&BsxParseConfig::xml(),
		)
		.unwrap();
		let mut world = World::new();
		let part =
			OoxmlNode::spawn_part(&mut world, "part.xml", &nodes).unwrap();
		let children = world.entity(part).get::<Children>().unwrap().to_vec();
		world
			.entity(children[0])
			.get::<OoxmlNode>()
			.unwrap()
			.clone()
			.xpect_eq(OoxmlNode::Instruction("xml version=\"1.0\"".into()));
		let root = children[2];
		let element = |entity: Entity| {
			world
				.entity(entity)
				.get::<OoxmlNode>()
				.unwrap()
				.element()
				.unwrap()
		};
		element(root).is("urn:test", "root").xpect_true();
		element(root).attribute(None, "a").xpect_eq(Some("1 & 2"));
		world.entity(root).contains::<Element>().xpect_false();
		let inner = world.entity(root).get::<Children>().unwrap().to_vec();
		element(inner[0])
			.attribute(Some(OoxmlNamespace::XML), "space")
			.xpect_eq(Some("preserve"));
		// an unprefixed element takes the default namespace, an unprefixed
		// attribute none
		element(inner[1]).is("urn:default", "Other").xpect_true();
		element(inner[1])
			.attribute(Some("urn:test"), "b")
			.xpect_eq(Some("x"));
		// CDATA is text, its spelling lexical
		world
			.entity(inner[2])
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("<raw>"));
	}
}
