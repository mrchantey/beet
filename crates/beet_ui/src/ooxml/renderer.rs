use super::word_writer::WordWriter;
use super::workbook_writer::WorkbookWriter;
use super::writer::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// Writes a document read from a Word file or a workbook back to its own
/// media type, by re-projection: every [`OoxmlNode`] as it says, and what an
/// edit made with none by the reverse of the mapping. An edit is a [`Value`]
/// differing from the one the file held. A slide deck is read, never
/// written. Every part the tree holds is written, and every other part is
/// kept as read.
///
/// # Word
///
/// Edited text keeps its spaces, a checkbox's glyph is written as its
/// `checked` says, a newly checked box bold and highlighted so the mark
/// survives an upload that drops the control, and every cell keeps a
/// paragraph. What an edit added reverses as follows:
///
/// - **`p`, `h1` to `h6`, `li`, `pre`**: a `w:p` with the properties of
///   the nearest paragraph of its own tag the file holds, a sibling first,
///   else the nearest in its part, so a new `<h2>` takes an `<h2>`'s style
///   and a new `<li>` joins a list. With none, a heading is written with its
///   [`ParagraphStyle::heading`], and any other with its first sibling
///   paragraph's properties, an `li` with a warning since no numbering is
///   invented. A `pre`'s lines are kept by breaks.
/// - **Words where a paragraph belongs**, ie straight under the body or in
///   a cell: each run of them and of the phrasing elements around them in a
///   new paragraph, as above, whitespace at its edges left out.
/// - **Text**: a run in the look of its paragraph's model's first run,
///   adding `w:b` inside a new `strong` or `b`, `w:i` inside an `em` or `i`,
///   and a yellow `w:highlight` inside a `mark`. New words inside a run the
///   file holds split it, their own run in its look between what came
///   before and what came after.
/// - **`br`**: a run holding a `w:br`.
/// - **`a href`** to an address outside the file: a `w:hyperlink` of an
///   external relationship its part holds, every link to one address
///   sharing one, its runs in the look of the nearest link the file holds.
///   A link whose `href` changed points at a relationship for its new
///   address, the old one kept.
/// - **`tr`, `td`, `th`** in a table the file holds: a `w:tr` with the
///   properties of the nearest row, never a repeated header, and a `w:tc`
///   with those of the cell in its column in the nearest other row, else of
///   the cell before it, never merged down and spanning its `colspan`. The
///   table's grid widens to its widest row.
///
/// What does not reverse is written as its text, its structure dropped, and
/// the render logs one warning naming it: a new `table` (its cells' words
/// written as paragraphs), `img`, `ins` or `del`, and a link to a place in
/// the file, `href="#.."`, which Word writes as a bookmark anchor the reader
/// does not project either. A `caption` is never written, since a Word
/// table has none and the one a reader sees is its number. Any other new
/// element, ie a `span` or a `ul`, is written through as its content.
///
/// # Workbook
///
/// An edited cell's value is written as a person typing it would: a number
/// when it parses as one, a formula when led by `=`, an inline string
/// otherwise. A gap's cell is created with its row's or column's style, a
/// locked cell refused, and the book marked to recalculate on open, since
/// nothing here computes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OoxmlRenderer;

impl Installer for OoxmlRenderer {
	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
}

impl DocRenderer for OoxmlRenderer {
	fn media_types(&self) -> Vec<MediaType> {
		vec![MediaType::Docx, MediaType::Xlsx]
	}

	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		let root = cx.entity;
		let package = cx
			.world
			.entity(root)
			.get::<OoxmlPackage>()
			.cloned()
			.ok_or_else(|| {
				bevyhow!("the document was not read from an Office file")
			})?;
		let media_type = package.media_type().clone();
		cx.check_accepts(&[media_type.clone()])?;
		let world = &mut *cx.world;
		let mut file = package.open()?;
		match media_type {
			MediaType::Docx => WordWriter::reproject(world, root, &mut file)?,
			MediaType::Xlsx => {
				if WorkbookWriter::reproject(world, root)? {
					WorkbookWriter::recalculate(&mut file)?;
				}
			}
			other => {
				return Err(
					bevyhow!("a {other} file is read, never written").into()
				);
			}
		}
		let parts = world
			.with_state::<(Query<&Children>, Query<&OoxmlNode>), _>(
				|(children, nodes)| {
					children
						.iter_descendants_depth_first(root)
						.filter_map(|entity| match nodes.get(entity) {
							Ok(OoxmlNode::Part { path }) => {
								Some((entity, path.clone()))
							}
							_ => None,
						})
						.collect::<Vec<_>>()
				},
			);
		for (entity, path) in parts {
			let text = world
				.with_state::<OoxmlWriter, _>(|writer| writer.write(entity));
			let utf16 = file
				.data_at(&path)?
				.is_some_and(|bytes| bytes.starts_with(&[0xFF, 0xFE]));
			let bytes = match utf16 {
				true => [0xFF, 0xFE]
					.into_iter()
					.chain(text.encode_utf16().flat_map(u16::to_le_bytes))
					.collect(),
				false => text.into_bytes(),
			};
			file.set_data(&path, bytes)?;
		}
		MediaBytes::new(media_type, file.to_bytes()?).xok()
	}
}

#[cfg(test)]
pub(crate) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	pub(crate) fn parse(bytes: &MediaBytes) -> (World, Entity) {
		let mut world =
			(TemplatePlugin, DocumentPlugin, ParsePlugin).into_world();
		let root = world.spawn_empty().id();
		DocParsers::parse(ParseContext::new(
			&mut world.entity_mut(root),
			bytes,
		))
		.unwrap();
		(world, root)
	}

	pub(crate) fn write(
		world: &mut World,
		root: Entity,
		media_type: MediaType,
	) -> MediaBytes {
		DocRenderers::render(
			world,
			root,
			&RequestParts::default().with_accept(media_type),
		)
		.unwrap()
	}

	/// Every XML part of a file in canonical form: its element tree with
	/// attributes sorted, its text, comments and instructions as they are,
	/// and lexical form, ie an empty element's spelling, set aside.
	pub(crate) fn canonical(bytes: &MediaBytes) -> Vec<(SmolStr, String)> {
		let file = OoxmlFile::open(bytes).unwrap();
		let mut parts = file
			.parts()
			.iter()
			.filter_map(|part| {
				let path = file.path(part)?;
				let nodes = OoxmlFile::read_xml(file.data(part).ok()??).ok()?;
				let mut out = String::new();
				for node in &nodes {
					write_canonical(node, &mut out);
				}
				Some((path, out))
			})
			.collect::<Vec<_>>();
		parts.sort();
		parts
	}

	fn write_canonical(node: &BsxNode, out: &mut String) {
		match node {
			BsxNode::Element(element) => {
				let mut attributes = element
					.attributes
					.iter()
					.map(|attribute| match &attribute.value {
						AttrValue::Str(value) => {
							format!(" {}={value:?}", attribute.key)
						}
						_ => format!(" {}", attribute.key),
					})
					.collect::<Vec<_>>();
				attributes.sort();
				out.push_str(&format!(
					"<{}{}>",
					element.tag,
					attributes.concat()
				));
				for child in &element.children {
					write_canonical(child, out);
				}
				out.push_str(&format!("</{}>", element.tag));
			}
			BsxNode::Text(text) | BsxNode::CData(text) => {
				out.push_str(&format!("{text:?}"))
			}
			BsxNode::Comment(text) => out.push_str(&format!("<!--{text}-->")),
			BsxNode::ProcessingInstruction(text) => {
				out.push_str(&format!("<?{text}?>"))
			}
			BsxNode::Doctype(text) => {
				out.push_str(&format!("<!DOCTYPE {text}>"))
			}
			BsxNode::Expr(_) => {}
		}
	}

	pub(crate) fn cells(world: &mut World, root: Entity) -> Vec<String> {
		CellText::listing(world, root)
			.iter()
			.map(|cell| cell.text.to_string())
			.collect()
	}

	/// Runs one edit on `world`'s document, its writes applied after.
	pub(crate) fn edit<O>(
		world: &mut World,
		func: impl FnOnce(&mut DocumentTreeEdit) -> O,
	) -> O {
		world.with_state::<DocumentTreeEdit, _>(|mut edit| func(&mut edit))
	}

	#[beet_core::test]
	fn untouched_files_write_back_render_identical() {
		for bytes in [
			super::super::word::test::form(),
			super::super::workbook_writer::test::workbook(),
		] {
			let (mut world, root) = parse(&bytes);
			let written = write(&mut world, root, bytes.media_type().clone());
			canonical(&written).xpect_eq(canonical(&bytes));
		}
	}

	/// The text node holding `words`.
	fn text_node(world: &mut World, words: &str) -> Entity {
		world
			.query::<(Entity, &Value)>()
			.iter(world)
			.find(|(_, value)| value.as_str().ok() == Some(words))
			.map(|(entity, _)| entity)
			.unwrap()
	}

	/// The part at `path` of `bytes` as text.
	pub(crate) fn part(bytes: &MediaBytes, path: &str) -> String {
		let file = OoxmlFile::open(bytes).unwrap();
		String::from_utf8(file.data_at(path).unwrap().unwrap().to_vec())
			.unwrap()
	}

	/// An edit is a value differing from its read: every value re-inserted
	/// as it was is none, and one changed value rewrites that value alone.
	#[beet_core::test]
	fn edits_are_found_by_comparison() {
		for (bytes, path, words, before, after) in [
			(
				super::super::word::test::form(),
				"word/document.xml",
				"Name",
				"<w:t>Name</w:t>",
				"<w:t xml:space=\"preserve\">Named</w:t>",
			),
			(
				super::super::workbook_writer::test::workbook(),
				"xl/worksheets/sheet1.xml",
				"Enter it here",
				"<is><t>Enter it here</t></is>",
				"<is><t xml:space=\"preserve\">Named</t></is>",
			),
		] {
			let media_type = bytes.media_type().clone();
			let untouched = {
				let (mut world, root) = parse(&bytes);
				write(&mut world, root, media_type.clone())
			};
			let (mut world, root) = parse(&bytes);
			let values = world
				.query::<(Entity, &Value)>()
				.iter(&world)
				.map(|(entity, value)| (entity, value.clone()))
				.collect::<Vec<_>>();
			world.insert_batch(values);
			let reinserted = write(&mut world, root, media_type.clone());
			canonical(&reinserted).xpect_eq(canonical(&untouched));

			let (mut world, root) = parse(&bytes);
			let text = text_node(&mut world, words);
			world.entity_mut(text).insert(Value::str("Named"));
			part(&write(&mut world, root, media_type), path)
				.xpect_eq(part(&untouched, path).replacen(before, after, 1));
		}
	}
}
