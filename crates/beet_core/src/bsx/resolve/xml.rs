//! Building XML, the one markup grammar read as written, as plain markup.
//!
//! The build resolves nothing. An uppercase tag is an element like any other
//! and a `bx:` attribute an attribute, so a document of any vocabulary builds
//! whole and writes back as it was read.
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
}
