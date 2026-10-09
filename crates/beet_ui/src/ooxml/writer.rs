//! [`OoxmlWriter`], a part written as its file says it.
use crate::prelude::*;
use beet_core::prelude::*;

/// Writes one part as its [`OoxmlNode`]s say: an element from its name and
/// attributes, a reader's text where a node of the file holds it, and every
/// other node as it was read. An entity with no node is projection only, ie a
/// list's `<ul>`: written through, its children in place, and its own words,
/// ie a table's caption, never written. A part below the one being written is
/// written on its own.
#[derive(SystemParam)]
pub(crate) struct OoxmlWriter<'w, 's> {
	nodes: Query<
		'w,
		's,
		(
			Option<&'static OoxmlNode>,
			Option<&'static Value>,
			Has<Element>,
			Option<&'static Children>,
		),
	>,
}

impl OoxmlWriter<'_, '_> {
	/// The part at `part` as XML.
	pub fn write(&self, part: Entity) -> String {
		let mut out = String::new();
		for child in self.children(part) {
			self.write_node(child, true, &mut out);
		}
		out
	}

	/// Writes `entity`, `held` when a node of the file holds it, so its text
	/// is the file's.
	fn write_node(&self, entity: Entity, held: bool, out: &mut String) {
		let Ok((node, value, is_element, _)) = self.nodes.get(entity) else {
			return;
		};
		match (node, value) {
			(Some(OoxmlNode::Part { .. }), _) => {}
			(Some(OoxmlNode::Element(element)), _) => {
				out.push('<');
				out.push_str(&element.name);
				for attribute in &element.attributes {
					XmlWriter::write_attribute(
						&attribute.name,
						&attribute.value,
						out,
					);
				}
				out.push('>');
				let start = out.len();
				for child in self.children(entity) {
					self.write_node(child, true, out);
				}
				// an element that wrote nothing as `<name/>`
				match out.len() == start {
					true => {
						out.pop();
						out.push_str("/>");
					}
					false => {
						out.push_str("</");
						out.push_str(&element.name);
						out.push('>');
					}
				}
			}
			(Some(OoxmlNode::Text(text)), _) => {
				out.push_str(&XmlWriter::escape_text(text))
			}
			(Some(OoxmlNode::Comment(text)), _) => {
				out.push_str("<!--");
				out.push_str(text);
				out.push_str("-->");
			}
			(Some(OoxmlNode::Instruction(text)), _) => {
				out.push_str("<?");
				out.push_str(text);
				out.push_str("?>");
			}
			(None, Some(value)) if held && !is_element => {
				out.push_str(&XmlWriter::escape_text(&value.to_string()))
			}
			// projection only: its children in place, its own words unwritten
			(None, _) => {
				for child in self.children(entity) {
					self.write_node(child, false, out);
				}
			}
		}
	}

	fn children(&self, entity: Entity) -> Vec<Entity> {
		self.nodes
			.get(entity)
			.ok()
			.and_then(|(.., children)| children)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// A projection is written through: its own words never written, a
	/// node no reader reads written though, and a nested part left to be
	/// written on its own.
	#[beet_core::test]
	fn writes_a_part_through_its_projection() {
		let mut world = World::new();
		let part = OoxmlNode::spawn_part(
			&mut world,
			"part.xml",
			&BsxNode::parse_document(
				"<w:p xmlns:w=\"urn:w\"><w:r><w:t>one</w:t></w:r><w:instr>PAGE</w:instr><w:n/></w:p>",
				&BsxParseConfig::xml(),
			)
			.unwrap(),
		)
		.unwrap();
		let paragraph = world.entity(part).get::<Children>().unwrap()[0];
		let children =
			world.entity(paragraph).get::<Children>().unwrap().to_vec();
		let (run, instruction, nested) =
			(children[0], children[1], children[2]);
		// the instruction's text kept for the writer alone
		let text = world.entity(instruction).get::<Children>().unwrap()[0];
		world
			.entity_mut(text)
			.remove::<Value>()
			.insert(OoxmlNode::Text("PAGE".into()));
		world.entity_mut(nested).insert(OoxmlNode::Part {
			path: "nested.xml".into(),
		});
		// a projection: the run inside a `strong`, and a caption of the
		// projection's own words
		world.entity_mut(paragraph).insert(Element::new("p"));
		let strong = world.spawn(Element::new("strong")).id();
		world.entity_mut(paragraph).insert_child(0, strong);
		world.entity_mut(run).insert(ChildOf(strong));
		world.spawn((Element::new("caption"), ChildOf(paragraph), children![
			Value::str("not written")
		]));
		world
			.with_state::<OoxmlWriter, _>(|writer| writer.write(part))
			.xpect_eq(
				"<w:p xmlns:w=\"urn:w\"><w:r><w:t>one</w:t></w:r><w:instr>PAGE</w:instr></w:p>",
			);
	}
}
