//! Lowering XML, the one markup grammar read as written, as plain markup.
//!
//! The lowering resolves nothing. An uppercase tag is an element like any
//! other and a `bx:` attribute an attribute, so a document of any vocabulary
//! lowers whole and writes back as it was read.
use super::lowering::*;
use crate::prelude::*;
use beet_core::prelude::*;

impl BsxParser {
	/// Lower `nodes` as plain markup: an element an [`Element`] named as
	/// written with its attributes as [`Attribute`] entities, text a
	/// [`Value`], and every other node its own component, all under a bare
	/// root. An expression has no xml form.
	pub(in crate::bsx) fn lower_markup(
		nodes: &[BsxNode],
	) -> Result<ParsedDocument> {
		let parsers = DocParsers::default();
		let types = parsers.types().read();
		Lowering::lower_plain(nodes, &parsers, &types)
	}
}

impl<'a> Lowering<'a> {
	/// Lower `el` onto the entity keyed `key` as the markup it is.
	pub fn plain_element(
		&mut self,
		el: &BsxElement,
		key: &EntityKey,
	) -> Result {
		self.insert(key, &Element::new(el.tag.clone()))?;
		for attr in &el.attributes {
			let mut components = EntityBody::default()
				.with_typed(&Attribute::new(attr.key.clone()))?;
			if let AttrValue::Str(value) = &attr.value {
				components.insert_typed(&Value::Str(value.into()))?;
			}
			self.attribute(key, &attr.key, components, &attr.span)?;
		}
		self.children(&el.children, key, None)
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	const XML: &str = "<?xml version=\"1.0\"?>\n<t:root xmlns:t=\"urn:test\" xmlns=\"urn:default\" a=\"1 &amp; 2\"><t:p xml:space=\"preserve\"> one </t:p><Other t:b=\"x\"/><![CDATA[<raw>]]><!-- note --></t:root>";

	#[beet_core::test]
	fn builds_markup_as_written() {
		let mut world = ScenePlugin::world();
		let body = DocParsers::of(&world)
			.body(&MediaBytes::new_str(MediaType::Xml, XML))
			.unwrap();
		let root = world.spawn_template(body).unwrap().id();
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
