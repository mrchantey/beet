//! Building XML, the one markup grammar read as written: a plain document as
//! markup, or one part of a source package as source nodes.
//!
//! Neither build resolves anything. An uppercase tag is an element like any
//! other and a `bx:` attribute an attribute, so a document of any vocabulary
//! builds whole and writes back as it was read.
use crate::prelude::*;

impl BsxNode {
	/// Spawns `nodes` under `parent` as plain markup: an element an [`Element`]
	/// named as written with its attributes as [`Attribute`] entities, text a
	/// [`Value`], and every other node its own component, so an `.xml` file
	/// builds like any page and every renderer reads it.
	pub fn spawn_markup(nodes: &[BsxNode], parent: &mut EntityWorldMut) {
		let parent_id = parent.id();
		parent.world_scope(|world| {
			for node in nodes {
				Self::spawn_markup_node(node, parent_id, world);
			}
		});
	}

	/// Spawns `nodes` under `parent` with their source identity, the form a
	/// part of an Office file is read in: an element a [`SourceElement`] with
	/// its name's and attributes' prefixes resolved against the declarations
	/// in scope, text a [`Value`], and every other node its own component. No
	/// element carries an [`Element`] yet: what a node means to a reader is
	/// the format's projection to add.
	pub fn spawn_source(nodes: &[BsxNode], parent: &mut EntityWorldMut) {
		let parent_id = parent.id();
		let mut scopes = NamespaceScopes::default();
		parent.world_scope(|world| {
			for node in nodes {
				Self::spawn_source_node(node, parent_id, &mut scopes, world);
			}
		});
	}

	fn spawn_markup_node(node: &BsxNode, parent: Entity, world: &mut World) {
		let BsxNode::Element(element) = node else {
			if let Some(mut leaf) = Self::spawn_leaf(node, world) {
				leaf.insert(ChildOf(parent));
			}
			return;
		};
		let entity = world
			.spawn((ChildOf(parent), Element::new(element.tag.clone())))
			.id();
		for attribute in &element.attributes {
			let mut spawned = world.spawn((
				AttributeOf::new(entity),
				Attribute::new(attribute.key.clone()),
			));
			if let AttrValue::Str(value) = &attribute.value {
				spawned.insert(Value::Str(value.into()));
			}
		}
		for child in &element.children {
			Self::spawn_markup_node(child, entity, world);
		}
	}

	fn spawn_source_node(
		node: &BsxNode,
		parent: Entity,
		scopes: &mut NamespaceScopes,
		world: &mut World,
	) {
		let BsxNode::Element(element) = node else {
			if let Some(mut leaf) = Self::spawn_leaf(node, world) {
				leaf.insert(ChildOf(parent));
			}
			return;
		};
		let source = scopes.open(element);
		let entity = world.spawn((ChildOf(parent), source)).id();
		for child in &element.children {
			Self::spawn_source_node(child, entity, scopes, world);
		}
		scopes.close();
	}

	/// A node that is no element as its own entity: text a [`Value`], the
	/// rest their components. An expression has no xml form, so spawns
	/// nothing.
	fn spawn_leaf<'w>(
		node: &BsxNode,
		world: &'w mut World,
	) -> Option<EntityWorldMut<'w>> {
		match node {
			BsxNode::Text(text) => world.spawn(Value::Str(text.into())),
			BsxNode::Comment(content) => world.spawn(Comment::new(content)),
			BsxNode::Doctype(value) => world.spawn(Doctype::new(value)),
			BsxNode::CData(content) => world.spawn(CData::new(content)),
			BsxNode::ProcessingInstruction(content) => {
				world.spawn(ProcessingInstruction::new(content))
			}
			BsxNode::Expr(_) | BsxNode::Element(_) => return None,
		}
		.xsome()
	}
}

/// The prefix bindings in force at each open element, innermost last.
#[derive(Default)]
struct NamespaceScopes {
	scopes: Vec<Vec<(SmolStr, SmolStr)>>,
}

impl NamespaceScopes {
	/// Opens `element`'s scope and resolves its name and attributes in it.
	fn open(&mut self, element: &BsxElement) -> SourceElement {
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
		SourceElement {
			name: element.tag.as_str().into(),
			namespace: self.resolve(prefix),
			attributes: attributes
				.into_iter()
				.map(|(key, value)| SourceAttribute {
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
			return Some(SourceElement::XML_NAMESPACE.into());
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

	const XML: &str = "<?xml version=\"1.0\"?>\n<t:root xmlns:t=\"urn:test\" xmlns=\"urn:default\" a=\"1 &amp; 2\"><t:p xml:space=\"preserve\"> one </t:p><Other t:b=\"x\"/><![CDATA[<raw>]]><!-- note --></t:root>";

	fn parse() -> Vec<BsxNode> {
		BsxNode::parse_document(XML, &BsxParseConfig::xml()).unwrap()
	}

	#[crate::test]
	fn builds_markup_as_written() {
		let mut world = World::new();
		let mut root = world.spawn_empty();
		BsxNode::spawn_markup(&parse(), &mut root);
		let root = root.id();
		let children = world.entity(root).get::<Children>().unwrap().to_vec();
		world
			.entity(children[0])
			.get::<ProcessingInstruction>()
			.unwrap()
			.target()
			.xpect_eq("xml");
		// the line break after the declaration is text, as written
		world
			.entity(children[1])
			.get::<Value>()
			.unwrap()
			.clone()
			.xpect_eq(Value::str("\n"));
		let element = children[2];
		world
			.entity(element)
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("t:root");
		world.with_state::<AttributeQuery, _>(|query| {
			query
				.find(element, "a")
				.map(|(_, value)| value.clone())
				.xpect_eq(Some(Value::str("1 & 2")));
		});
		let inner = world.entity(element).get::<Children>().unwrap().to_vec();
		// an uppercase tag is an element like any other
		world
			.entity(inner[1])
			.get::<Element>()
			.unwrap()
			.tag()
			.xpect_eq("Other");
		world
			.entity(inner[2])
			.get::<CData>()
			.unwrap()
			.0
			.as_str()
			.xpect_eq("<raw>");
	}

	#[crate::test]
	fn resolves_source_namespaces() {
		let mut world = World::new();
		let mut root = world.spawn_empty();
		BsxNode::spawn_source(&parse(), &mut root);
		let root = root.id();
		let element = world.entity(root).get::<Children>().unwrap()[2];
		let source = world.entity(element).get::<SourceElement>().unwrap();
		source.is("urn:test", "root").xpect_true();
		source.attribute(None, "a").xpect_eq(Some("1 & 2"));
		world.entity(element).contains::<Element>().xpect_false();
		let inner = world.entity(element).get::<Children>().unwrap().to_vec();
		let paragraph = world.entity(inner[0]).get::<SourceElement>().unwrap();
		paragraph
			.attribute(Some(SourceElement::XML_NAMESPACE), "space")
			.xpect_eq(Some("preserve"));
		// an unprefixed element takes the default namespace, an unprefixed
		// attribute none
		let other = world.entity(inner[1]).get::<SourceElement>().unwrap();
		other.is("urn:default", "Other").xpect_true();
		other.attribute(Some("urn:test"), "b").xpect_eq(Some("x"));
	}
}
