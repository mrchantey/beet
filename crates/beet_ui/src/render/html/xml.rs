//! The XML mode of [`HtmlRenderer`]: a tree written as the markup it was read
//! from.
use beet_core::prelude::*;

/// Writes a tree as XML: a node from its source identity when it has one and
/// from its [`Element`] and [`Attribute`]s otherwise.
///
/// Inside a source tree, below a [`SourceElement`], an entity with no source
/// identity is projection only, ie a list's `<ul>`: the writer passes through
/// it and writes its children in place, and writes text, a comment or any
/// other leaf only where a source element holds it, so the projection's own
/// words are never written. A
/// [`SourceText`] is written wherever it sits, children are written in their
/// [`SourceOrder`] where a projection moved them, and a [`SourcePart`] below
/// the one being written is another part, so is skipped.
#[derive(SystemParam)]
pub(crate) struct XmlWriter<'w, 's> {
	nodes: Query<'w, 's, XmlNode<'static>>,
	attributes: Query<'w, 's, (&'static Attribute, Option<&'static Value>)>,
	orders: Query<'w, 's, &'static SourceOrder>,
}

type XmlNode<'a> = (
	Option<&'a SourceElement>,
	Option<&'a Element>,
	Option<&'a Attributes>,
	Option<&'a Value>,
	Option<&'a SourceText>,
	Option<&'a Comment>,
	Option<&'a Doctype>,
	Option<&'a CData>,
	Option<&'a ProcessingInstruction>,
	Option<&'a Children>,
	Has<SourcePart>,
);

/// Where a node sits: below a source element at all, and directly.
#[derive(Clone, Copy, Default)]
struct Place {
	in_source: bool,
	parent_is_source: bool,
}

impl XmlWriter<'_, '_> {
	/// `entity` and its subtree as XML.
	pub fn write(&self, entity: Entity) -> String {
		let mut out = String::new();
		self.write_node(entity, true, Place::default(), &mut out);
		out
	}

	fn write_node(
		&self,
		entity: Entity,
		is_start: bool,
		place: Place,
		out: &mut String,
	) {
		let Ok((
			source,
			element,
			attributes,
			value,
			source_text,
			comment,
			doctype,
			cdata,
			instruction,
			children,
			is_part,
		)) = self.nodes.get(entity)
		else {
			return;
		};
		if is_part && !is_start {
			return;
		}
		// a leaf is written where a source element holds it, or anywhere in
		// plain markup, so a projection's own words and comments never are
		let written = place.parent_is_source || !place.in_source;
		if let Some(doctype) = doctype.filter(|_| written) {
			out.push_str("<!DOCTYPE ");
			out.push_str(doctype);
			out.push('>');
		}
		if let Some(comment) = comment.filter(|_| written) {
			out.push_str("<!--");
			out.push_str(comment);
			out.push_str("-->");
		}
		if let Some(instruction) = instruction.filter(|_| written) {
			out.push_str("<?");
			out.push_str(instruction);
			out.push_str("?>");
		}
		if let Some(cdata) = cdata.filter(|_| written) {
			out.push_str("<![CDATA[");
			out.push_str(cdata);
			out.push_str("]]>");
		}
		if let Some(text) = source_text {
			out.push_str(&Self::escape_text(text));
		}
		if let Some(value) = value
			&& source.is_none()
			&& element.is_none()
			&& written
		{
			out.push_str(&Self::escape_text(&value.to_string()));
		}
		let children = children.map(|children| self.ordered(children));
		match (source, element) {
			(Some(source), _) => {
				self.open(&source.name, out);
				for attribute in &source.attributes {
					Self::write_attribute(
						&attribute.name,
						&attribute.value,
						out,
					);
				}
				self.close_children(
					&source.name,
					children,
					Place {
						in_source: true,
						parent_is_source: true,
					},
					out,
				);
			}
			(None, Some(element)) if !place.in_source => {
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
				self.close_children(element.tag(), children, place, out);
			}
			// projection only, or a container: its children in place
			_ => {
				for child in children.into_iter().flatten() {
					self.write_node(
						child,
						false,
						Place {
							parent_is_source: false,
							..place
						},
						out,
					);
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
		place: Place,
		out: &mut String,
	) {
		out.push('>');
		let start = out.len();
		for child in children.into_iter().flatten() {
			self.write_node(child, false, place, out);
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

	/// The children in source order: by [`SourceOrder`] where a projection
	/// recorded one, else as they stand.
	fn ordered(&self, children: &Children) -> Vec<Entity> {
		let mut children = children.iter().enumerate().collect::<Vec<_>>();
		if children
			.iter()
			.any(|(_, child)| self.orders.contains(*child))
		{
			children.sort_by_key(|(index, child)| {
				self.orders
					.get(*child)
					.map_or(*index as u32, |order| order.0)
			});
		}
		children.into_iter().map(|(_, child)| child).collect()
	}

	fn write_attribute(name: &str, value: &str, out: &mut String) {
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
	fn escape_text(text: &str) -> String {
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

	/// In a source tree an entity with no source identity is projection
	/// only: written through, its own words never written, a source text
	/// written though no reader sees it, children restored to their source
	/// order, and a nested part left to be written on its own.
	#[beet_core::test]
	fn writes_a_source_tree_through_its_projection() {
		let mut world = World::new();
		let mut root = world.spawn(SourcePart::new("part.xml"));
		BsxNode::spawn_source(
			&BsxNode::parse_document(
				"<w:p xmlns:w=\"urn:w\"><w:r><w:t>one</w:t></w:r><w:instr>PAGE</w:instr><w:n/></w:p>",
				&BsxParseConfig::xml(),
			)
			.unwrap(),
			&mut root,
		);
		let root = root.id();
		let paragraph = world.entity(root).get::<Children>().unwrap()[0];
		let children =
			world.entity(paragraph).get::<Children>().unwrap().to_vec();
		let (run, instruction, nested) =
			(children[0], children[1], children[2]);
		// the instruction's text kept for the writer alone
		let text = world.entity(instruction).get::<Children>().unwrap()[0];
		world
			.entity_mut(text)
			.remove::<Value>()
			.insert(SourceText::new("PAGE"));
		world
			.entity_mut(nested)
			.insert(SourcePart::new("nested.xml"));
		// a projection: the run moved into a `strong` after its siblings, and
		// a caption of the projection's own words
		world.entity_mut(paragraph).insert(Element::new("p"));
		let strong = world
			.spawn((Element::new("strong"), ChildOf(paragraph)))
			.id();
		world.entity_mut(run).insert(ChildOf(strong));
		world.spawn((Element::new("caption"), ChildOf(paragraph), children![
			Value::str("not written")
		]));
		for (order, entity) in [(0, strong), (1, instruction), (2, nested)] {
			world.entity_mut(entity).insert(SourceOrder(order));
		}
		HtmlRenderer::xml()
			.render(&mut RenderContext::new(root, &mut world))
			.unwrap()
			.to_string()
			.xpect_eq(
				"<w:p xmlns:w=\"urn:w\"><w:r><w:t>one</w:t></w:r><w:instr>PAGE</w:instr></w:p>",
			);
	}
}
