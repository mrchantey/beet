use super::source_tree::*;
use crate::prelude::*;
use beet_core::prelude::*;
use bevy::ecs::change_detection::Tick;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

/// Writes a document read from an Office file back to its own media type, by
/// re-projection: every source node from its source components, and what an
/// edit made with no source identity by the reverse of the mapping, new
/// content in its neighbour's style.
///
/// A Word file writes a new paragraph with its cell's first paragraph's
/// properties and new words in a run of that paragraph's first look, marks
/// edited text to keep its spaces, writes a checkbox's glyph as its
/// `checked` says, a newly checked box bold and highlighted so the mark
/// survives an upload that drops the control, and keeps a paragraph in every
/// cell. A workbook writes an edited cell's value, a number when it parses
/// as one and an inline string otherwise, creates a gap's cell with its
/// row's or column's style, refuses a locked cell or a formula, and marks
/// the book to recalculate on open, since nothing here computes. Every part
/// the tree holds is written, and every other part is kept as read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct OoxmlRenderer;

impl NodeRenderer for OoxmlRenderer {
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
			MediaType::Docx => {
				WordWriter::reproject(world, root, package.parsed())
			}
			MediaType::Xlsx => {
				if WorkbookWriter::reproject(world, root, package.parsed())? {
					WorkbookWriter::recalculate(&mut file)?;
				}
			}
			_ => {}
		}
		let parts = world
			.with_state::<(Query<&Children>, Query<&SourcePart>), _>(
				|(children, parts)| {
					children
						.iter_descendants_depth_first(root)
						.filter_map(|entity| {
							parts
								.get(entity)
								.ok()
								.map(|part| (entity, part.path.clone()))
						})
						.collect::<Vec<_>>()
				},
			);
		for (entity, path) in parts {
			let text =
				world.with_state::<XmlWriter, _>(|writer| writer.write(entity));
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

/// A deep copy of a source subtree under `parent`, ie a model paragraph's
/// properties for a new one.
fn clone_source(world: &mut World, entity: Entity, parent: Entity) -> Entity {
	let copy = world.spawn(ChildOf(parent)).id();
	if let Some(source) = world.entity(entity).get::<SourceElement>().cloned() {
		world.entity_mut(copy).insert(source);
	}
	if let Some(text) = world.entity(entity).get::<SourceText>().cloned() {
		world.entity_mut(copy).insert(text);
	}
	if let Some(value) = world.entity(entity).get::<Value>().cloned() {
		world.entity_mut(copy).insert(value);
	}
	let children = world
		.entity(entity)
		.get::<Children>()
		.map(|children| children.to_vec())
		.unwrap_or_default();
	for child in children {
		clone_source(world, child, copy);
	}
	copy
}

/// Whether `entity`'s [`Value`] changed after the read ended at `parsed`.
fn edited(world: &World, entity: Entity, parsed: Tick) -> bool {
	world
		.entity(entity)
		.get_change_ticks::<Value>()
		.is_some_and(|ticks| ticks.is_changed(parsed, world.read_change_tick()))
}

/// The re-projection of a Word file's edits.
struct WordWriter;

/// The order `CT_RPr` fixes for a run's properties, which Word holds a file
/// to.
const RUN_PROPERTY_ORDER: [&str; 40] = [
	"rStyle",
	"rFonts",
	"b",
	"bCs",
	"i",
	"iCs",
	"caps",
	"smallCaps",
	"strike",
	"dstrike",
	"outline",
	"shadow",
	"emboss",
	"imprint",
	"noProof",
	"snapToGrid",
	"vanish",
	"webHidden",
	"color",
	"spacing",
	"w",
	"kern",
	"position",
	"sz",
	"szCs",
	"highlight",
	"u",
	"effect",
	"bdr",
	"shd",
	"fitText",
	"vertAlign",
	"rtl",
	"cs",
	"em",
	"lang",
	"eastAsianLayout",
	"specVanish",
	"oMath",
	"rPrChange",
];

impl WordWriter {
	/// The empty and the ticked checkbox glyphs, when a control names none.
	const BOX: char = '\u{2610}';
	const TICKED: char = '\u{2612}';

	fn reproject(world: &mut World, root: Entity, parsed: Tick) {
		let models = Self::paragraphs(world, root);
		Self::texts(world, root, &models, parsed);
		Self::checkboxes(world, root);
		Self::cells(world, root);
	}

	/// Gives every new paragraph its source identity, a `w:p` with its first
	/// source sibling's properties, answering each paragraph's model, the
	/// paragraph whose runs its new words take their look from.
	fn paragraphs(world: &mut World, root: Entity) -> HashMap<Entity, Entity> {
		let found = world.with_state::<(
			SourceTree,
			Query<(Entity, &Element), Without<SourceElement>>,
			Query<&Children>,
		), _>(|(tree, elements, children)| {
			children
				.iter_descendants_depth_first(root)
				.filter_map(|entity| {
					let (_, element) = elements.get(entity).ok()?;
					let is_paragraph = ReaderText::BLOCKS
						.contains(&element.tag())
						&& element.tag() != "pre";
					let parent = tree.parent(entity)?;
					// a new paragraph sits among source nodes
					let in_source = tree
						.ancestors(entity)
						.into_iter()
						.any(|ancestor| tree.element(ancestor).is_some());
					(is_paragraph && in_source).then(|| {
						let model = tree
							.children(parent)
							.into_iter()
							.find(|sibling| tree.is(*sibling, Ns::WORD, "p"));
						let properties = model.and_then(|model| {
							tree.child(model, Ns::WORD, "pPr")
						});
						let named =
							tree.ancestors(entity).into_iter().find_map(
								|ancestor| tree.element(ancestor).cloned(),
							);
						(entity, model, properties, named)
					})
				})
				.collect::<Vec<_>>()
		});
		let mut models = HashMap::default();
		for (entity, model, properties, named) in found {
			let source = match named {
				Some(named) => named.sibling(Ns::WORD, "p"),
				None => SourceElement::new("w:p", Some(Ns::WORD.into())),
			};
			world.entity_mut(entity).insert(source);
			if let Some(properties) = properties {
				let copy = clone_source(world, properties, entity);
				world.entity_mut(entity).insert_child(0, copy);
			}
			models.insert(entity, model.unwrap_or(entity));
		}
		models
	}

	/// Puts every new text node in a run of its paragraph's first look, and
	/// keeps the spaces of every edited text.
	fn texts(
		world: &mut World,
		root: Entity,
		models: &HashMap<Entity, Entity>,
		parsed: Tick,
	) {
		let preserve = SourceAttribute::new(
			"xml:space",
			Some(SourceElement::XML_NAMESPACE.into()),
			"preserve",
		);
		let texts = world.with_state::<(
			SourceTree,
			Query<&Children>,
			Query<(), (With<Value>, Without<Element>, Without<SourceElement>)>,
		), _>(|(tree, children, texts)| {
			children
				.iter_descendants_depth_first(root)
				.filter(|entity| texts.contains(*entity))
				.filter_map(|text| {
					let parent = tree.parent(text)?;
					// inside a template, never written by a reader's edit
					let in_t = tree.element(parent).is_some_and(|parent| {
						matches!(
							parent.local_name(),
							"t" | "delText" | "instrText"
						)
					});
					let paragraph = tree.ancestor(text, Ns::WORD, "p")?;
					Some((text, parent, in_t, paragraph))
				})
				.collect::<Vec<_>>()
		});
		for (text, parent, in_t, paragraph) in texts {
			if in_t {
				if edited(world, text, parsed)
					&& let Some(mut source) =
						world.entity_mut(parent).get_mut::<SourceElement>()
				{
					source.set_attribute(preserve.clone());
				}
				continue;
			}
			let model = models.get(&paragraph).copied().unwrap_or(paragraph);
			let (run_properties, named) =
				world.with_state::<SourceTree, _>(|tree| {
					let runs = tree.descendants_named(model, Ns::WORD, "r");
					let run = runs
						.iter()
						.find(|run| {
							tree.descendant(**run, Ns::WORD, "t").is_some()
						})
						.or(runs.first())
						.copied();
					(
						run.and_then(|run| tree.child(run, Ns::WORD, "rPr")),
						tree.element(paragraph).cloned(),
					)
				});
			let named = named.unwrap_or_else(|| {
				SourceElement::new("w:p", Some(Ns::WORD.into()))
			});
			let index = world
				.entity(parent)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == text)
				})
				.unwrap_or_default();
			let run = world.spawn(named.sibling(Ns::WORD, "r")).id();
			if let Some(properties) = run_properties {
				clone_source(world, properties, run);
			}
			let mut written = named.sibling(Ns::WORD, "t");
			written.set_attribute(preserve.clone());
			let element = world.spawn((written, ChildOf(run))).id();
			world.entity_mut(element).add_child(text);
			world.entity_mut(parent).insert_child(index, run);
		}
	}

	/// Writes each checkbox control's state as its `checked` says: the
	/// control's own value, its glyph, and a newly checked one's glyph bold
	/// and highlighted.
	fn checkboxes(world: &mut World, root: Entity) {
		let boxes = world
			.with_state::<(SourceTree, ReaderText, Query<&Children>), _>(
				|(tree, text, children)| {
					children
						.iter_descendants_depth_first(root)
						.filter(|entity| tree.is(*entity, Ns::WORD, "sdt"))
						.filter_map(|control| {
							let checked = text.checkbox(control)?;
							let properties =
								tree.child(control, Ns::WORD, "sdtPr")?;
							let checkbox = tree.descendant(
								properties,
								Ns::WORD_2010,
								"checkbox",
							)?;
							let state =
								tree.child(checkbox, Ns::WORD_2010, "checked");
							let was = state
								.and_then(|state| {
									tree.attribute(
										state,
										Some(Ns::WORD_2010),
										"val",
									)
								})
								.is_some_and(|value| {
									matches!(value, "1" | "true")
								});
							let glyph = |local: &str, default: char| {
								tree.child(checkbox, Ns::WORD_2010, local)
									.and_then(|glyph| {
										tree.attribute(
											glyph,
											Some(Ns::WORD_2010),
											"val",
										)
									})
									.and_then(|code| {
										u32::from_str_radix(code, 16).ok()
									})
									.and_then(char::from_u32)
									.unwrap_or(default)
							};
							let content =
								tree.child(control, Ns::WORD, "sdtContent");
							let glyphs = content
								.map(|content| {
									tree.descendants_named(
										content,
										Ns::WORD,
										"t",
									)
									.into_iter()
									.flat_map(|written| tree.children(written))
									.collect::<Vec<_>>()
								})
								.unwrap_or_default();
							let runs = content
								.map(|content| {
									tree.descendants_named(
										content,
										Ns::WORD,
										"r",
									)
								})
								.unwrap_or_default();
							(checked != was).then(|| {
								(
									checked,
									checkbox,
									state,
									glyph("checkedState", Self::TICKED),
									glyph("uncheckedState", Self::BOX),
									glyphs,
									runs,
									tree.element(checkbox).cloned(),
								)
							})
						})
						.collect::<Vec<_>>()
				},
			);
		for (checked, checkbox, state, ticked, empty, glyphs, runs, named) in
			boxes
		{
			let Some(named) = named else { continue };
			let value = SourceAttribute::new(
				format!("{}:val", named.prefix()),
				named.namespace.clone(),
				if checked { "1" } else { "0" },
			);
			match state {
				Some(state) => {
					if let Some(mut source) =
						world.entity_mut(state).get_mut::<SourceElement>()
					{
						source.set_attribute(value);
					}
				}
				// the schema puts `checked` first
				None => {
					let mut source = named.sibling(Ns::WORD_2010, "checked");
					source.set_attribute(value);
					let state = world.spawn(source).id();
					world.entity_mut(checkbox).insert_child(0, state);
				}
			}
			let (from, to) = match checked {
				true => (empty, ticked),
				false => (ticked, empty),
			};
			for glyph in glyphs {
				if let Some(mut text) =
					world.entity_mut(glyph).get_mut::<SourceText>()
				{
					text.0 = text.0.replacen(from, &to.to_string(), 1);
				}
			}
			if checked {
				for run in runs {
					Self::mark(world, run);
				}
			}
		}
	}

	/// Makes a run bold and highlighted yellow, its properties in schema
	/// order.
	fn mark(world: &mut World, run: Entity) {
		let (properties, named, present) =
			world.with_state::<SourceTree, _>(|tree| {
				let properties = tree.child(run, Ns::WORD, "rPr");
				let present = properties
					.map(|properties| {
						tree.children(properties)
							.into_iter()
							.filter_map(|child| tree.element(child))
							.map(|element| element.local_name().to_owned())
							.collect::<Vec<_>>()
					})
					.unwrap_or_default();
				(properties, tree.element(run).cloned(), present)
			});
		let Some(named) = named else { return };
		let properties = properties.unwrap_or_else(|| {
			let properties = world.spawn(named.sibling(Ns::WORD, "rPr")).id();
			world.entity_mut(run).insert_child(0, properties);
			properties
		});
		let rank = |local: &str| {
			RUN_PROPERTY_ORDER
				.iter()
				.position(|name| *name == local)
				.unwrap_or(RUN_PROPERTY_ORDER.len())
		};
		for (local, value) in [("b", None), ("highlight", Some("yellow"))] {
			if present.iter().any(|existing| existing == local) {
				continue;
			}
			let mut source = named.sibling(Ns::WORD, local);
			if let Some(value) = value {
				source.set_attribute(SourceAttribute::new(
					format!("{}:val", named.prefix()),
					Some(Ns::WORD.into()),
					value,
				));
			}
			let siblings = world.with_state::<SourceTree, _>(|tree| {
				tree.children(properties)
					.into_iter()
					.map(|child| {
						tree.element(child)
							.map(|element| element.local_name().to_owned())
							.unwrap_or_default()
					})
					.collect::<Vec<_>>()
			});
			let index = siblings
				.iter()
				.position(|existing| rank(existing) > rank(local))
				.unwrap_or(siblings.len());
			let property = world.spawn(source).id();
			world.entity_mut(properties).insert_child(index, property);
		}
	}

	/// Keeps a paragraph in every cell, since Word holds a cell to one.
	fn cells(world: &mut World, root: Entity) {
		let empty = world.with_state::<(SourceTree, Query<&Children>), _>(
			|(tree, children)| {
				children
					.iter_descendants_depth_first(root)
					.filter(|entity| tree.is(*entity, Ns::WORD, "tc"))
					.filter(|cell| {
						tree.descendant(*cell, Ns::WORD, "p").is_none()
					})
					.filter_map(|cell| {
						Some((cell, tree.element(cell)?.clone()))
					})
					.collect::<Vec<_>>()
			},
		);
		for (cell, named) in empty {
			world.spawn((named.sibling(Ns::WORD, "p"), ChildOf(cell)));
		}
	}
}

/// The re-projection of a workbook's edits.
struct WorkbookWriter;

impl WorkbookWriter {
	/// Writes every edited cell, answering whether any was.
	fn reproject(
		world: &mut World,
		root: Entity,
		parsed: Tick,
	) -> Result<bool> {
		let cells = world.with_state::<(
			Query<&Children>,
			Query<
				(&SheetCellAddress, Option<&SheetCellStyle>),
				Without<CoveredBy>,
			>,
			Query<&SourceElement>,
			Query<(), With<Value>>,
		), _>(|(children, cells, sources, values)| {
			children
				.iter_descendants_depth_first(root)
				.filter_map(|entity| {
					let (address, style) = cells.get(entity).ok()?;
					let texts = children
						.iter_descendants(entity)
						.chain([entity])
						.filter(|node| values.contains(*node))
						.collect::<Vec<_>>();
					Some((
						entity,
						address.clone(),
						style.map(|style| style.0),
						sources.contains(entity),
						texts,
					))
				})
				.collect::<Vec<_>>()
		});
		let mut any = false;
		for (cell, address, style, is_source, texts) in cells {
			let changed = match is_source {
				true => texts.iter().any(|text| edited(world, *text, parsed)),
				// a gap is a cell once anything is written in it
				false => !texts.is_empty(),
			};
			if !changed {
				continue;
			}
			if world.entity(cell).contains::<CellLocked>() {
				bevybail!(
					"{address} is locked; only an unlocked cell takes a value"
				);
			}
			let value = world
				.with_state::<ReaderText, _>(|text| text.own_text(cell))
				.trim()
				.to_string();
			if value.starts_with('=') {
				bevybail!("{address}: a formula is not a value");
			}
			Self::write(world, cell, &address, style, &value);
			any = true;
		}
		Ok(any)
	}

	/// Writes `value` as `cell`'s only content: a number in a `v`, anything
	/// else an inline string, a gap becoming the `c` it stands for.
	fn write(
		world: &mut World,
		cell: Entity,
		address: &SheetCellAddress,
		style: Option<u32>,
		value: &str,
	) {
		let named = match world.entity(cell).get::<SourceElement>().cloned() {
			Some(named) => named,
			None => {
				let row = world
					.entity(cell)
					.get::<ChildOf>()
					.and_then(|parent| {
						world.entity(parent.parent()).get::<SourceElement>()
					})
					.cloned()
					.unwrap_or_else(|| {
						SourceElement::new("row", Some(Ns::SPREADSHEET.into()))
					});
				let mut named = row.sibling(Ns::SPREADSHEET, "c");
				named.set_attribute(SourceAttribute::new(
					"r",
					None,
					address.a1(),
				));
				if let Some(style) = style.filter(|style| *style != 0) {
					named.set_attribute(SourceAttribute::new(
						"s",
						None,
						style.to_string(),
					));
				}
				world.entity_mut(cell).insert(named.clone());
				named
			}
		};
		world.entity_mut(cell).despawn_children();
		let mut source = named.clone();
		let is_number = Self::is_number(value);
		match is_number {
			true => {
				source.remove_attribute(None, "t");
			}
			false => source.set_attribute(SourceAttribute::new(
				"t",
				None,
				"inlineStr",
			)),
		}
		world.entity_mut(cell).insert(source);
		match is_number {
			true => {
				world.spawn((
					named.sibling(Ns::SPREADSHEET, "v"),
					ChildOf(cell),
					children![Value::str(value)],
				));
			}
			false => {
				let mut text = named.sibling(Ns::SPREADSHEET, "t");
				text.set_attribute(SourceAttribute::new(
					"xml:space",
					Some(SourceElement::XML_NAMESPACE.into()),
					"preserve",
				));
				world.spawn((
					named.sibling(Ns::SPREADSHEET, "is"),
					ChildOf(cell),
					children![(text, children![Value::str(value)])],
				));
			}
		}
	}

	/// Whether `text` is a plain decimal, `-?digits` with an optional
	/// fraction.
	fn is_number(text: &str) -> bool {
		let digits = text.strip_prefix('-').unwrap_or(text);
		let (whole, fraction) = digits.split_once('.').unwrap_or((digits, "0"));
		!whole.is_empty()
			&& !fraction.is_empty()
			&& whole
				.chars()
				.chain(fraction.chars())
				.all(|char| char.is_ascii_digit())
	}

	/// Marks the workbook to recalculate every formula on open.
	fn recalculate(file: &mut OoxmlFile) -> Result {
		let Some(workbook) = file
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::WorkbookPart(_)))
		else {
			return Ok(());
		};
		let path = file.path(&workbook).unwrap_or_default();
		let Some(nodes) = file.xml_at(&path)? else {
			return Ok(());
		};
		let mut world = World::new();
		let part = world.spawn(SourcePart::new(path.clone())).id();
		BsxNode::spawn_source(&nodes, &mut world.entity_mut(part));
		let (book, calculation, after) =
			world.with_state::<SourceTree, _>(|tree| {
				let book = tree
					.children(part)
					.into_iter()
					.find(|child| tree.is(*child, Ns::SPREADSHEET, "workbook"));
				let calculation = book.and_then(|book| {
					tree.child(book, Ns::SPREADSHEET, "calcPr")
				});
				// `calcPr` follows these in the schema's sequence
				let after = book.and_then(|book| {
					tree.children(book).into_iter().rposition(|child| {
						tree.element(child).is_some_and(|element| {
							matches!(
								element.local_name(),
								"fileVersion"
									| "fileSharing" | "workbookPr"
									| "workbookProtection" | "bookViews"
									| "sheets" | "functionGroups"
									| "externalReferences" | "definedNames"
							)
						})
					})
				});
				(book, calculation, after)
			});
		let Some(book) = book else {
			return Ok(());
		};
		let full = SourceAttribute::new("fullCalcOnLoad", None, "1");
		match calculation {
			Some(calculation) => {
				if let Some(mut source) =
					world.entity_mut(calculation).get_mut::<SourceElement>()
				{
					source.set_attribute(full);
				}
			}
			None => {
				let named =
					world.entity(book).get::<SourceElement>().cloned().unwrap();
				let mut source = named.sibling(Ns::SPREADSHEET, "calcPr");
				source.set_attribute(full);
				let calculation = world.spawn(source).id();
				world.entity_mut(book).insert_child(
					after.map_or(0, |index| index + 1),
					calculation,
				);
			}
		}
		let text =
			world.with_state::<XmlWriter, _>(|writer| writer.write(part));
		file.set_data(&path, text.into_bytes())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	fn parse(bytes: &MediaBytes) -> (World, Entity) {
		let mut world =
			(TemplatePlugin, DocumentPlugin, ParsePlugin).into_world();
		let root = world.spawn_empty().id();
		MediaParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(root), bytes))
			.unwrap();
		(world, root)
	}

	fn write(
		world: &mut World,
		root: Entity,
		media_type: MediaType,
	) -> MediaBytes {
		MediaRenderer::default()
			.render(
				&mut RenderContext::new(root, world)
					.with_accepts(vec![media_type]),
			)
			.unwrap()
	}

	/// Every XML part of a file in canonical form: its element tree with
	/// attributes sorted, its text, comments and instructions as they are,
	/// and lexical form, ie an empty element's spelling, set aside.
	fn canonical(bytes: &MediaBytes) -> Vec<(SmolStr, String)> {
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

	fn document(bytes: &MediaBytes) -> String {
		let file = OoxmlFile::open(bytes).unwrap();
		String::from_utf8(
			file.data_at("word/document.xml").unwrap().unwrap().to_vec(),
		)
		.unwrap()
	}

	fn cells(world: &mut World, root: Entity) -> Vec<String> {
		world
			.with_state::<TableCells, _>(|cells| cells.listing(root))
			.iter()
			.map(|cell| cell.text.to_string())
			.collect()
	}

	#[beet_core::test]
	fn untouched_files_write_back_render_identical() {
		for bytes in [super::super::word::test::form(), workbook()] {
			let (mut world, root) = parse(&bytes);
			let written = write(&mut world, root, bytes.media_type().clone());
			canonical(&written).xpect_eq(canonical(&bytes));
		}
	}

	#[beet_core::test]
	fn fills_a_word_form() {
		let (mut world, root) = parse(&super::super::word::test::form());
		let cell = |text| CellAddress::parse(text).unwrap();
		SetText {
			cell: cell("t1r1c2"),
			text: "Acme Stalls\nSecond line".into(),
		}
		.apply_to(&mut world, root)
		.unwrap();
		AppendText {
			cell: cell("t1r1c1"),
			text: "appended".into(),
		}
		.apply_to(&mut world, root)
		.unwrap();
		CheckBox {
			label: "Surveys".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		CheckBox {
			label: "Nothing".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(0);
		RemoveParagraphs {
			text: "Please delete this sentence".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		ReplaceText {
			old: "(Insert name)".into(),
			new: "Acme".into(),
		}
		.apply_to(&mut world, root)
		.unwrap()
		.xpect_eq(1);
		SetText {
			cell: cell("t9r1c1"),
			text: "x".into(),
		}
		.apply_to(&mut world, root)
		.xpect_err();
		RemoveParagraphs { text: " ".into() }
			.apply_to(&mut world, root)
			.xpect_err();

		let written = write(&mut world, root, MediaType::Docx);
		let (mut world, root) = parse(&written);
		cells(&mut world, root).xpect_eq(vec![
			"Name / appended".to_string(),
			"Acme Stalls / Second line".into(),
			String::new(),
			"[x] Surveys".into(),
		]);
		document(&written)
			.xpect_contains("<w14:checked w14:val=\"1\"/>")
			.xpect_contains("\u{2612}")
			// bold goes before the size, highlight after, as the schema orders
			.xpect_contains("<w:rPr><w:b/><w:sz w:val=\"20\"/><w:highlight w:val=\"yellow\"/></w:rPr>")
			// a new paragraph keeps the cell's paragraph style
			.xpect_contains("<w:pPr><w:jc w:val=\"left\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Acme Stalls</w:t></w:r></w:p><w:p><w:pPr><w:jc w:val=\"left\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Second line</w:t>")
			// and new words the look of the paragraph's first
			.xpect_contains("<w:r><w:rPr><w:sz w:val=\"20\"/></w:rPr><w:t xml:space=\"preserve\">appended</w:t></w:r>")
			.xpect_contains("The business is </w:t></w:r><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t xml:space=\"preserve\">Acme</w:t>");
	}

	/// A protected sheet: `A1` a locked label, `D3:F3` an unlocked merged
	/// field holding a prompt, `D4` an unlocked formula, and column `G`
	/// unlocked by its column style where no cell is written.
	fn workbook() -> MediaBytes {
		OoxmlFile::workbook(
			"<xf/><xf><protection locked=\"0\"/></xf>",
			&[(
				"Start Here",
				"<cols><col min=\"7\" max=\"7\" style=\"1\"/></cols><sheetData>\
				 <row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Business name</t></is></c></row>\
				 <row r=\"3\"><c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t>Enter it here</t></is></c>\
				 <c r=\"E3\" s=\"1\"/><c r=\"F3\" s=\"1\"/><c r=\"H3\"/></row>\
				 <row r=\"4\"><c r=\"D4\" s=\"1\"><f>1+1</f><v>2</v></c></row>\
				 </sheetData><sheetProtection sheet=\"1\"/><mergeCells count=\"1\"><mergeCell ref=\"D3:F3\"/></mergeCells>",
			)],
		)
		.unwrap()
	}

	#[beet_core::test]
	fn lists_and_sets_unlocked_cells() {
		let (mut world, root) = parse(&workbook());
		world
			.with_state::<TableCells, _>(|cells| cells.listing(root))
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| Start Here!D3 | Enter it here |".to_string(),
				"| Start Here!E3 | Enter it here |".into(),
				"| Start Here!F3 | Enter it here |".into(),
				"| Start Here!G3 | (empty) |".into(),
				"| Start Here!D4 | =1+1 |".into(),
			]);
		let set = |world: &mut World, cell: &str, text: &str| {
			SetText {
				cell: CellAddress::parse(cell).unwrap(),
				text: text.into(),
			}
			.apply_to(world, root)
		};
		// a covered cell writes its range's first
		let written = set(&mut world, "Start Here!E3", "Acme Stalls").unwrap();
		world
			.entity(written)
			.get::<SheetCellAddress>()
			.unwrap()
			.to_string()
			.xpect_eq("Start Here!D3");
		set(&mut world, "Start Here!G3", "120.5").unwrap();
		set(&mut world, "Start Here!A1", "x")
			.unwrap_err()
			.to_string()
			.xpect_contains("is locked");
		set(&mut world, "Start Here!G3", "=2")
			.unwrap_err()
			.to_string()
			.xpect_contains("a formula is not a value");
		set(&mut world, "Nowhere!A1", "x").xpect_err();

		let bytes = write(&mut world, root, MediaType::Xlsx);
		let file = OoxmlFile::open(&bytes).unwrap();
		let sheet = String::from_utf8(
			file.data_at("xl/worksheets/sheet1.xml")
				.unwrap()
				.unwrap()
				.to_vec(),
		)
		.unwrap();
		sheet
			.xpect_contains("<c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Acme Stalls</t></is></c>")
			// the gap's cell, created with its column's style
			.xpect_contains("<c r=\"G3\" s=\"1\"><v>120.5</v></c><c r=\"H3\"/>");
		String::from_utf8(
			file.data_at("xl/workbook.xml").unwrap().unwrap().to_vec(),
		)
		.unwrap()
		.xpect_contains("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/>");
		let (mut world, root) = parse(&bytes);
		cells(&mut world, root).xpect_eq(vec![
			"Acme Stalls".to_string(),
			"Acme Stalls".into(),
			"Acme Stalls".into(),
			"120.5".into(),
			"=1+1".into(),
		]);
	}
}
