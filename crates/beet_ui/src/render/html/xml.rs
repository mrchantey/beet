//! The XML mode of [`HtmlRenderer`]: a tree written as the markup it reads
//! as.
use beet_core::prelude::*;

/// Writes a tree as XML: an element from its [`Element`] and [`Attribute`]s,
/// text from its [`Value`], and a comment, doctype, CDATA section or
/// processing instruction as itself. An entity with no element, ie a parse
/// root or a node only an Office file's writer reads, is written through,
/// its children in place.
#[derive(SystemParam)]
pub(crate) struct XmlWriter<'w, 's> {
	nodes: Query<'w, 's, XmlNode<'static>>,
	attributes: Query<'w, 's, (&'static Attribute, Option<&'static Value>)>,
}

type XmlNode<'a> = (
	Option<&'a Element>,
	Option<&'a Attributes>,
	Option<&'a Value>,
	Option<&'a Comment>,
	Option<&'a Doctype>,
	Option<&'a CData>,
	Option<&'a ProcessingInstruction>,
	Option<&'a Children>,
);

impl XmlWriter<'_, '_> {
	/// `entity` and its subtree as XML.
	pub fn write(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.write_node(entity, &mut out);
		out
	}

	fn write_node(&self, entity: Entity, out: &mut String) {
		let Ok((
			element,
			attributes,
			value,
			comment,
			doctype,
			cdata,
			instruction,
			children,
		)) = self.nodes.get(entity)
		else {
			return;
		};
		if let Some(doctype) = doctype {
			out.push_str("<!DOCTYPE ");
			out.push_str(doctype);
			out.push('>');
		}
		if let Some(comment) = comment {
			out.push_str("<!--");
			out.push_str(comment);
			out.push_str("-->");
		}
		if let Some(instruction) = instruction {
			out.push_str("<?");
			out.push_str(instruction);
			out.push_str("?>");
		}
		if let Some(cdata) = cdata {
			out.push_str("<![CDATA[");
			out.push_str(cdata);
			out.push_str("]]>");
		}
		let children = children.map(|children| children.to_vec());
		match (element, value) {
			(Some(element), _) => {
				self.open(element.tag(), out);
				for attribute in attributes.iter().flat_map(|list| list.iter())
				{
					if let Ok((key, value)) = self.attributes.get(attribute) {
						let value = value
							.filter(|value| !value.is_null())
							.map(ToString::to_string)
							.unwrap_or_default();
						Self::write_attribute(key, &value, out);
					}
				}
				self.close_children(element.tag(), children, out);
			}
			(None, Some(value)) => {
				out.push_str(&Self::escape_text(&value.to_string()))
			}
			(None, None) => {
				for child in children.into_iter().flatten() {
					self.write_node(child, out);
				}
			}
		}
	}

	fn open(&self, name: &str, out: &mut String) {
		out.push('<');
		out.push_str(name);
	}

	/// Writes the children after an open tag's attributes and closes it, an
	/// element that wrote nothing as `<name/>`.
	fn close_children(
		&self,
		name: &str,
		children: Option<Vec<Entity>>,
		out: &mut String,
	) {
		out.push('>');
		let start = out.len();
		for child in children.into_iter().flatten() {
			self.write_node(child, out);
		}
		match out.len() == start {
			true => {
				out.pop();
				out.push_str("/>");
			}
			false => {
				out.push_str("</");
				out.push_str(name);
				out.push('>');
			}
		}
	}

	/// Writes ` name="value"`, the value escaped for an attribute.
	pub(crate) fn write_attribute(name: &str, value: &str, out: &mut String) {
		out.push(' ');
		out.push_str(name);
		out.push_str("=\"");
		for char in value.chars() {
			match char {
				'&' => out.push_str("&amp;"),
				'<' => out.push_str("&lt;"),
				'"' => out.push_str("&quot;"),
				// whitespace other than a space as a reference, since a
				// literal one reads back as a space
				'\t' => out.push_str("&#x9;"),
				'\n' => out.push_str("&#xA;"),
				'\r' => out.push_str("&#xD;"),
				char => out.push(char),
			}
		}
		out.push('"');
	}

	/// Text escaped for XML character data.
	pub(crate) fn escape_text(text: &str) -> String {
		let mut out = String::with_capacity(text.len());
		for char in text.chars() {
			match char {
				'&' => out.push_str("&amp;"),
				'<' => out.push_str("&lt;"),
				'>' => out.push_str("&gt;"),
				char => out.push(char),
			}
		}
		out
	}
}

#[cfg(test)]
#[cfg(feature = "bsx")]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// Every lexical form the writer chooses is the one Office writes, so a
	/// document written that way round-trips byte for byte: the declaration,
	/// namespaces, whitespace kept where it is text, references, CDATA,
	/// comments and processing instructions.
	const XML: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\r\n<t:root xmlns:t=\"urn:t\" xmlns:u=\"urn:u\" a=\"1 &amp; 2 &lt; 3 &quot;q&quot;\" b=\"x&#xA;y\"><t:p xml:space=\"preserve\">  two  spaces </t:p>\n\t<u:empty/><![CDATA[<raw> & ]]><!-- note --><?pi data?>&lt;tag&gt; &amp; text<Upper Mixed=\"1\"/></t:root>";

	fn parse(xml: &str) -> (World, Entity) {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		MediaParser::new()
			.parse(ParseContext::new(
				&mut world.entity_mut(entity),
				&MediaBytes::new(MediaType::Xml, xml.as_bytes()),
			))
			.unwrap();
		(world, entity)
	}

	fn write(world: &mut World, entity: Entity) -> String {
		MediaRenderer::default()
			.render(
				&mut RenderContext::new(entity, world)
					.with_accepts(vec![MediaType::Xml]),
			)
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn round_trips_plain_xml() {
		let (mut world, entity) = parse(XML);
		write(&mut world, entity).xpect_eq(XML);
	}

	/// An edit to the markup is written: a value changed and an element
	/// removed.
	#[beet_core::test]
	fn writes_edits() {
		let (mut world, entity) = parse("<a><b>one</b><c/></a>");
		let root = world.entity(entity).get::<Children>().unwrap()[0];
		let children = world.entity(root).get::<Children>().unwrap().to_vec();
		let text = world.entity(children[0]).get::<Children>().unwrap()[0];
		world.entity_mut(text).insert(Value::str("two & three"));
		world.entity_mut(children[1]).despawn();
		write(&mut world, entity).xpect_eq("<a><b>two &amp; three</b></a>");
	}
}
