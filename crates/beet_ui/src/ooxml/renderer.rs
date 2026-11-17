use super::node::OoxmlRead;
use super::ooxml_query::*;
use super::workbook::SheetCellStyle;
use super::writer::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

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
/// - **`p`, `h1` to `h6`, `li`**: a `w:p` with the properties of the
///   nearest paragraph of its own tag the file holds, a sibling first, else
///   the nearest in its part, so a new `<h2>` takes an `<h2>`'s style and a
///   new `<li>` joins a list. With none, a heading is written with its
///   [`ParagraphStyle::heading`], and a `p` or an `li` with its first
///   sibling paragraph's properties, an `li` with a warning since no
///   numbering is invented.
/// - **Text**: a run in the look of its paragraph's model's first run,
///   adding `w:b` inside a new `strong` or `b`, `w:i` inside an `em` or `i`,
///   and a yellow `w:highlight` inside a `mark`.
/// - **`a href`** to an address outside the file: a `w:hyperlink` of an
///   external relationship its part holds, every link to one address
///   sharing one, its runs in the look of the nearest link the file holds.
///   A link whose `href` changed points at a relationship for its new
///   address, the old one kept.
///
/// What does not reverse is written as its text, its structure dropped, and
/// the render logs one warning naming it: a new `table` (its cells written
/// as paragraphs), `img`, `ins` or `del`, and a link to a place in the file,
/// `href="#.."`, which Word writes as a bookmark anchor the reader does not
/// project either. A row or cell added to a table the file holds is named
/// too, its words lost unless a new paragraph inside it holds them. Any
/// other new element, ie a `span` or a `ul`, is written through as its
/// content.
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

/// A deep copy of a part's subtree under `parent`, ie a model paragraph's
/// properties for a new one.
fn clone_nodes(world: &mut World, entity: Entity, parent: Entity) -> Entity {
	let copy = world.spawn(ChildOf(parent)).id();
	if let Some(node) = world.entity(entity).get::<OoxmlNode>().cloned() {
		world.entity_mut(copy).insert(node);
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
		clone_nodes(world, child, copy);
	}
	copy
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

	/// The elements an edit may add that a Word file is not written with,
	/// each written as its text.
	const DROPPED: [&str; 4] = ["table", "img", "ins", "del"];

	/// A table's rows and cells, dropped when an edit adds one to a table
	/// the file holds.
	const TABLE_PARTS: [&str; 3] = ["tr", "td", "th"];

	fn reproject(
		world: &mut World,
		root: Entity,
		file: &mut OoxmlFile,
	) -> Result {
		Self::warn_dropped(world, root);
		let models = Self::paragraphs(world, root);
		let link_runs = Self::links(world, root, file)?;
		Self::texts(world, root, &models, &link_runs);
		Self::checkboxes(world, root);
		Self::cells(world, root);
		Ok(())
	}

	/// Whether `href` is an address outside the file, which a
	/// `w:hyperlink`'s relationship holds.
	fn is_external(href: &str) -> bool {
		!href.is_empty() && !href.starts_with('#')
	}

	/// Logs one warning naming what [`dropped`](Self::dropped) finds.
	fn warn_dropped(world: &mut World, root: Entity) {
		let dropped = Self::dropped(world, root);
		if !dropped.is_empty() {
			warn!(
				"a Word file is not written with the new `{}`: their structure dropped, their words kept where a paragraph holds them",
				dropped.join("`, `")
			);
		}
	}

	/// What an edit added whose structure is dropped, each named once: a new
	/// [`DROPPED`](Self::DROPPED) element, a link to a place in the file, and
	/// a row or cell added to a table the file holds.
	fn dropped(world: &mut World, root: Entity) -> Vec<&'static str> {
		world.with_state::<(OoxmlQuery, ElementQuery), _>(|(tree, elements)| {
			let mut dropped = Vec::<&'static str>::new();
			for entity in tree.descendants(root) {
				let Ok(view) = elements.get(entity) else {
					continue;
				};
				let in_held_table = || {
					tree.ancestors(entity)
						.into_iter()
						.any(|ancestor| tree.is(ancestor, Ns::WORD, "tbl"))
				};
				let name = match view.tag() {
					"a" if view.attribute_string("href").starts_with('#') => {
						"a href=\"#..\""
					}
					_ if tree.element(entity).is_some() => continue,
					tag => match (
						Self::DROPPED.iter().find(|name| **name == tag),
						Self::TABLE_PARTS.iter().find(|name| **name == tag),
					) {
						(Some(name), _) => *name,
						// a row or cell added to a table the file holds
						(None, Some(name)) if in_held_table() => *name,
						_ => continue,
					},
				};
				if tree.within_source(entity) && !dropped.contains(&name) {
					dropped.push(name);
				}
			}
			dropped
		})
	}

	/// Whether `entity`, an element an edit added with `tag`, is written as
	/// a `w:p`: a paragraph, or a cell of a new table holding none.
	fn is_new_paragraph(
		tree: &OoxmlQuery,
		elements: &Query<&Element>,
		entity: Entity,
		tag: &str,
	) -> bool {
		let tag_of =
			|entity: Entity| elements.get(entity).ok().map(Element::tag);
		match tag {
			"pre" => false,
			tag if DocumentText::is_block(tag) => true,
			tag if Cells::is_cell_tag(tag) => {
				let new_table = tree
					.ancestors(entity)
					.into_iter()
					.find(|ancestor| tag_of(*ancestor) == Some("table"))
					.is_some_and(|table| tree.element(table).is_none());
				let holds_paragraph =
					tree.descendants(entity).into_iter().any(|descendant| {
						tag_of(descendant).is_some_and(DocumentText::is_block)
					});
				new_table && !holds_paragraph
			}
			_ => false,
		}
	}

	/// Makes every new paragraph a `w:p` with the properties of the nearest
	/// file-held paragraph of its own tag, a sibling first, else the nearest
	/// in its part. With none, a heading takes its `Heading<n>` style and
	/// any other its first sibling paragraph's properties. Answers each new
	/// paragraph's model, whose runs its new words take their look from.
	fn paragraphs(world: &mut World, root: Entity) -> HashMap<Entity, Entity> {
		let found = world.with_state::<(OoxmlQuery, Query<&Element>), _>(
			|(tree, elements)| {
				let order = DocumentOrder::new(&tree, root);
				let tag_of = |entity: Entity| {
					elements.get(entity).ok().map(Element::tag)
				};
				let held = tree
					.descendants(root)
					.into_iter()
					.filter(|entity| tree.is(*entity, Ns::WORD, "p"))
					.collect::<Vec<_>>();
				tree.descendants(root)
					.into_iter()
					.filter_map(|entity| {
						let tag = tag_of(entity)?;
						let is_new = tree.element(entity).is_none()
							&& tree.within_source(entity)
							&& Self::is_new_paragraph(
								&tree, &elements, entity, tag,
							);
						if !is_new {
							return None;
						}
						let siblings = tree
							.parent(entity)
							.map(|parent| tree.children(parent))
							.unwrap_or_default();
						let same_tag = |candidate: &Entity| {
							tree.is(*candidate, Ns::WORD, "p")
								&& tag_of(*candidate) == Some(tag)
						};
						let part = tree.part_of(entity);
						let model = order
							.nearest(
								entity,
								siblings.iter().copied().filter(same_tag),
							)
							.or_else(|| {
								order.nearest(
									entity,
									held.iter()
										.copied()
										.filter(same_tag)
										.filter(|candidate| {
											tree.part_of(*candidate) == part
										}),
								)
							});
						let first_sibling = siblings
							.iter()
							.copied()
							.find(|sibling| tree.is(*sibling, Ns::WORD, "p"));
						let properties = |paragraph: Entity| {
							tree.child(paragraph, Ns::WORD, "pPr")
						};
						let written =
							match (model, DocumentText::heading_level(tag)) {
								(Some(model), _) => NewParagraph::Like {
									model,
									properties: properties(model),
								},
								(None, Some(level)) => {
									NewParagraph::Heading(level)
								}
								(None, None) => NewParagraph::Fallback {
									model: first_sibling,
									properties: first_sibling
										.and_then(properties),
									is_item: tag == "li",
								},
							};
						let named =
							tree.ancestors(entity).into_iter().find_map(
								|ancestor| tree.element(ancestor).cloned(),
							);
						Some((entity, written, named))
					})
					.collect::<Vec<_>>()
			},
		);
		let mut models = HashMap::default();
		for (entity, written, named) in found {
			let named = named.unwrap_or_else(|| {
				OoxmlElement::new("w:p", Some(Ns::WORD.into()))
			});
			let paragraph = named.sibling(Ns::WORD, "p");
			world
				.entity_mut(entity)
				.insert(OoxmlNode::Element(paragraph.clone()));
			let (model, properties) = match written {
				NewParagraph::Like { model, properties } => {
					(Some(model), properties)
				}
				NewParagraph::Heading(level) => {
					let mut style = paragraph.sibling(Ns::WORD, "pStyle");
					style.set_attribute(style.own_attribute(
						"val",
						ParagraphStyle::heading(level).0,
					));
					let properties = world
						.spawn((
							OoxmlNode::Element(
								paragraph.sibling(Ns::WORD, "pPr"),
							),
							children![OoxmlNode::Element(style)],
						))
						.id();
					world.entity_mut(entity).insert_child(0, properties);
					(None, None)
				}
				NewParagraph::Fallback {
					model,
					properties,
					is_item,
				} => {
					if is_item {
						warn!(
							"a new list item has no list item to join: written as a paragraph, since no numbering is invented"
						);
					}
					(model, properties)
				}
			};
			if let Some(properties) = properties {
				let copy = clone_nodes(world, properties, entity);
				world.entity_mut(entity).insert_child(0, copy);
			}
			models.insert(entity, model.unwrap_or(entity));
		}
		models
	}

	/// Writes every new link to an address outside the file as a
	/// `w:hyperlink` of an external relationship its part holds, shared by
	/// every link to that address, points an existing link whose `href`
	/// changed at its new address, and writes a link to a place in the file
	/// as its text. Answers each new link's model run properties, those of
	/// the nearest link's run the file holds.
	fn links(
		world: &mut World,
		root: Entity,
		file: &mut OoxmlFile,
	) -> Result<HashMap<Entity, Entity>> {
		let found = world.with_state::<(OoxmlQuery, ElementQuery), _>(
			|(tree, elements)| {
				let order = DocumentOrder::new(&tree, root);
				let held = tree
					.descendants(root)
					.into_iter()
					.filter(|entity| tree.is(*entity, Ns::WORD, "hyperlink"))
					.collect::<Vec<_>>();
				tree.descendants(root)
					.into_iter()
					.filter(|entity| tree.within_source(*entity))
					.filter_map(|link| {
						let view = elements.get(link).ok()?;
						(view.tag() == "a").then_some(())?;
						let href = SmolStr::new(view.attribute_string("href"));
						let part = tree.part_of(link)?;
						let path = tree.path(part)?.clone();
						if tree.is(link, Ns::WORD, "hyperlink") {
							let id = tree
								.attribute(link, Some(Ns::RELATIONSHIPS), "id")
								.map(SmolStr::new);
							return Some(FoundLink::Held {
								link,
								href,
								path,
								id,
							});
						}
						let paragraph = tree.ancestor(link, Ns::WORD, "p")?;
						let model = order
							.nearest(
								link,
								held.iter().copied().filter(|candidate| {
									tree.part_of(*candidate) == Some(part)
								}),
							)
							.or_else(|| held.first().copied())
							.and_then(|model| {
								tree.descendant(model, Ns::WORD, "r")
							})
							.and_then(|run| tree.child(run, Ns::WORD, "rPr"));
						Some(FoundLink::New {
							link,
							href,
							path,
							named: tree.element(paragraph)?.clone(),
							prefix: tree
								.prefix_for(paragraph, Ns::RELATIONSHIPS)
								.map(SmolStr::new),
							model,
						})
					})
					.collect::<Vec<_>>()
			},
		);
		let mut link_runs = HashMap::default();
		for found in found {
			match found {
				FoundLink::Held { link, href, .. }
					if !Self::is_external(&href) =>
				{
					// its runs stay, written in place
					world.entity_mut(link).remove::<OoxmlNode>();
				}
				FoundLink::Held {
					link,
					href,
					path,
					id,
				} => {
					let target = file.part_at(&path).and_then(|part| {
						id.as_ref()
							.and_then(|id| file.hyperlinks(&part).remove(id))
					});
					if target.as_ref() == Some(&href) {
						continue;
					}
					let id = file.hyperlink_to(&path, &href)?;
					if let Some(mut node) =
						world.entity_mut(link).get_mut::<OoxmlNode>()
						&& let Some(element) = node.element_mut()
					{
						// the name is kept, the value replaced
						element.set_attribute(OoxmlAttribute::new(
							"r:id",
							Some(Ns::RELATIONSHIPS.into()),
							id,
						));
					}
				}
				FoundLink::New { href, .. } if !Self::is_external(&href) => {}
				FoundLink::New {
					link,
					href,
					path,
					named,
					prefix,
					model,
				} => {
					let id = file.hyperlink_to(&path, &href)?;
					let mut hyperlink = named.sibling(Ns::WORD, "hyperlink");
					let prefix = match prefix {
						Some(prefix) => prefix,
						None => {
							hyperlink.attributes.push(OoxmlAttribute::new(
								"xmlns:r",
								None,
								Ns::RELATIONSHIPS,
							));
							"r".into()
						}
					};
					hyperlink.attributes.push(OoxmlAttribute::new(
						format!("{prefix}:id"),
						Some(Ns::RELATIONSHIPS.into()),
						id,
					));
					world
						.entity_mut(link)
						.insert(OoxmlNode::Element(hyperlink));
					if let Some(model) = model {
						link_runs.insert(link, model);
					}
				}
			}
		}
		Ok(link_runs)
	}

	/// Puts every new text node in a run of its paragraph's first look, or
	/// within a new link the look of the file's nearest link, adding what
	/// the new elements around it say, and keeps the spaces of every edited
	/// text.
	fn texts(
		world: &mut World,
		root: Entity,
		models: &HashMap<Entity, Entity>,
		link_runs: &HashMap<Entity, Entity>,
	) {
		let preserve =
			OoxmlAttribute::new("xml:space", Some(Ns::XML.into()), "preserve");
		let texts = world.with_state::<(
			OoxmlQuery,
			Query<&Element>,
			Query<(), (With<Value>, Without<Element>, Without<OoxmlNode>)>,
		), _>(|(tree, elements, texts)| {
			tree.descendants(root)
				.into_iter()
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
					let within = tree
						.ancestors(text)
						.into_iter()
						.take_while(|ancestor| *ancestor != paragraph)
						.collect::<Vec<_>>();
					let link_run = within
						.iter()
						.find_map(|ancestor| link_runs.get(ancestor))
						.copied();
					let mut looks = Vec::<RunProperty>::new();
					for tag in within
						.iter()
						.filter(|ancestor| tree.element(**ancestor).is_none())
						.filter_map(|ancestor| elements.get(*ancestor).ok())
						.map(Element::tag)
					{
						let look = match tag {
							tag if TextFormatting::is_bold(tag) => ("b", None),
							tag if TextFormatting::is_italic(tag) => {
								("i", None)
							}
							"mark" => ("highlight", Some("yellow")),
							_ => continue,
						};
						if !looks.contains(&look) {
							looks.push(look);
						}
					}
					Some((text, parent, in_t, paragraph, link_run, looks))
				})
				.collect::<Vec<_>>()
		});
		for (text, parent, in_t, paragraph, link_run, looks) in texts {
			if in_t {
				if OoxmlRead::edited(world, text)
					&& let Some(mut node) =
						world.entity_mut(parent).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.set_attribute(preserve.clone());
				}
				continue;
			}
			let model = models.get(&paragraph).copied().unwrap_or(paragraph);
			let (run_properties, named) =
				world.with_state::<OoxmlQuery, _>(|tree| {
					let runs = tree.descendants_named(model, Ns::WORD, "r");
					let run = runs
						.iter()
						.find(|run| {
							tree.descendant(**run, Ns::WORD, "t").is_some()
						})
						.or(runs.first())
						.copied();
					(
						link_run.or_else(|| {
							run.and_then(|run| tree.child(run, Ns::WORD, "rPr"))
						}),
						tree.element(paragraph).cloned(),
					)
				});
			let named = named.unwrap_or_else(|| {
				OoxmlElement::new("w:p", Some(Ns::WORD.into()))
			});
			let index = world
				.entity(parent)
				.get::<Children>()
				.and_then(|children| {
					children.iter().position(|child| child == text)
				})
				.unwrap_or_default();
			let run = world
				.spawn(OoxmlNode::Element(named.sibling(Ns::WORD, "r")))
				.id();
			if let Some(properties) = run_properties {
				clone_nodes(world, properties, run);
			}
			Self::add_properties(world, run, &looks);
			let mut written = named.sibling(Ns::WORD, "t");
			written.set_attribute(preserve.clone());
			let element = world
				.spawn((OoxmlNode::Element(written), ChildOf(run)))
				.id();
			world.entity_mut(element).add_child(text);
			world.entity_mut(parent).insert_child(index, run);
		}
	}

	/// Writes each checkbox control's state as its `checked` says: the
	/// control's own value, its glyph, and a newly checked one's glyph bold
	/// and highlighted.
	fn checkboxes(world: &mut World, root: Entity) {
		let boxes = world
			.with_state::<(OoxmlQuery, DocumentText, Query<&Children>), _>(
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
			let value =
				named.own_attribute("val", if checked { "1" } else { "0" });
			match state {
				Some(state) => {
					if let Some(mut node) =
						world.entity_mut(state).get_mut::<OoxmlNode>()
						&& let Some(element) = node.element_mut()
					{
						element.set_attribute(value);
					}
				}
				// the schema puts `checked` first
				None => {
					let mut element = named.sibling(Ns::WORD_2010, "checked");
					element.set_attribute(value);
					let state = world.spawn(OoxmlNode::Element(element)).id();
					world.entity_mut(checkbox).insert_child(0, state);
				}
			}
			let (from, to) = match checked {
				true => (empty, ticked),
				false => (ticked, empty),
			};
			for glyph in glyphs {
				if let Some(mut node) =
					world.entity_mut(glyph).get_mut::<OoxmlNode>()
					&& let OoxmlNode::Text(text) = &mut *node
				{
					*text = text.replacen(from, &to.to_string(), 1).into();
				}
			}
			if checked {
				for run in runs {
					Self::add_properties(world, run, &[
						("b", None),
						("highlight", Some("yellow")),
					]);
				}
			}
		}
	}

	/// Adds each of `properties` to `run`'s own, in the order the schema
	/// fixes: a property already present is kept, a toggle switched on.
	fn add_properties(
		world: &mut World,
		run: Entity,
		properties: &[RunProperty],
	) {
		if properties.is_empty() {
			return;
		}
		let (existing, named) = world.with_state::<OoxmlQuery, _>(|tree| {
			(tree.child(run, Ns::WORD, "rPr"), tree.element(run).cloned())
		});
		let Some(named) = named else { return };
		let container = existing.unwrap_or_else(|| {
			let container = world
				.spawn(OoxmlNode::Element(named.sibling(Ns::WORD, "rPr")))
				.id();
			world.entity_mut(run).insert_child(0, container);
			container
		});
		let rank = |local: &str| {
			RUN_PROPERTY_ORDER
				.iter()
				.position(|name| *name == local)
				.unwrap_or(RUN_PROPERTY_ORDER.len())
		};
		for (local, value) in properties {
			let present = world.with_state::<OoxmlQuery, _>(|tree| {
				tree.children(container)
					.into_iter()
					.map(|child| {
						let local = tree
							.element(child)
							.map(|element| element.local_name().to_owned())
							.unwrap_or_default();
						(child, local)
					})
					.collect::<Vec<_>>()
			});
			if let Some((property, _)) =
				present.iter().find(|(_, existing)| existing == local)
			{
				// a toggle written off, ie `w:b w:val="0"`, is switched on
				if value.is_none()
					&& let Some(mut node) =
						world.entity_mut(*property).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.remove_attribute(Some(Ns::WORD), "val");
				}
				continue;
			}
			let mut property = named.sibling(Ns::WORD, local);
			if let Some(value) = value {
				property.set_attribute(property.own_attribute("val", *value));
			}
			let index = present
				.iter()
				.position(|(_, existing)| rank(existing) > rank(local))
				.unwrap_or(present.len());
			let property = world.spawn(OoxmlNode::Element(property)).id();
			world.entity_mut(container).insert_child(index, property);
		}
	}

	/// Keeps a paragraph in every cell, since Word holds a cell to one.
	fn cells(world: &mut World, root: Entity) {
		let empty = world.with_state::<(OoxmlQuery, Query<&Children>), _>(
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
			world.spawn((
				OoxmlNode::Element(named.sibling(Ns::WORD, "p")),
				ChildOf(cell),
			));
		}
	}
}

/// A run property an edit's element adds, its local name and the value of
/// its `w:val`, a toggle when none, ie `("b", None)`.
type RunProperty = (&'static str, Option<&'static str>);

/// How a new paragraph is written.
enum NewParagraph {
	/// With the properties of a paragraph of its own tag, its model.
	Like {
		model: Entity,
		properties: Option<Entity>,
	},
	/// A heading with no model, by its style.
	Heading(u8),
	/// Any other with no model, by its first sibling paragraph.
	Fallback {
		model: Option<Entity>,
		properties: Option<Entity>,
		is_item: bool,
	},
}

/// A link found under a Word file's root.
enum FoundLink {
	/// A `w:hyperlink` the file holds, by its relationship's `id`.
	Held {
		link: Entity,
		href: SmolStr,
		path: SmolStr,
		id: Option<SmolStr>,
	},
	/// One an edit added inside the paragraph `named`, `prefix` bound to the
	/// relationships namespace there, `model` the run properties of the
	/// file's nearest link.
	New {
		link: Entity,
		href: SmolStr,
		path: SmolStr,
		named: OoxmlElement,
		prefix: Option<SmolStr>,
		model: Option<Entity>,
	},
}

/// Every entity under a root by its place in document order.
struct DocumentOrder(HashMap<Entity, usize>);

impl DocumentOrder {
	fn new(tree: &OoxmlQuery, root: Entity) -> Self {
		Self(
			tree.descendants(root)
				.into_iter()
				.enumerate()
				.map(|(index, entity)| (entity, index))
				.collect(),
		)
	}

	/// Of `candidates`, the last before `entity`, else the first after it.
	fn nearest(
		&self,
		entity: Entity,
		candidates: impl IntoIterator<Item = Entity>,
	) -> Option<Entity> {
		let at = self.0.get(&entity).copied().unwrap_or_default();
		let (before, after): (Vec<_>, Vec<_>) = candidates
			.into_iter()
			.filter_map(|candidate| Some((*self.0.get(&candidate)?, candidate)))
			.partition(|(index, _)| *index < at);
		before
			.into_iter()
			.max_by_key(|(index, _)| *index)
			.or_else(|| after.into_iter().min_by_key(|(index, _)| *index))
			.map(|(_, candidate)| candidate)
	}
}

/// The re-projection of a workbook's edits.
struct WorkbookWriter;

impl WorkbookWriter {
	/// Writes every edited cell, answering whether any was.
	fn reproject(world: &mut World, root: Entity) -> Result<bool> {
		let cells = world.with_state::<(
			Query<&Children>,
			Query<(&CellAddress, Option<&SheetCellStyle>)>,
			Query<&OoxmlNode>,
			Query<(), With<Value>>,
		), _>(|(children, cells, nodes, values)| {
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
						nodes.contains(entity),
						texts,
					))
				})
				.collect::<Vec<_>>()
		});
		let mut any = false;
		for (cell, address, style, is_written, texts) in cells {
			let changed = match is_written {
				true => {
					texts.iter().any(|text| OoxmlRead::edited(world, *text))
				}
				// a gap is a cell once anything is written in it
				false => !texts.is_empty(),
			};
			if !changed {
				continue;
			}
			if world.with_state::<Cells, _>(|cells| cells.is_locked(cell)) {
				bevybail!(
					"{address} is locked; only an unlocked cell takes a value"
				);
			}
			let value = world
				.with_state::<DocumentText, _>(|text| text.own_text(cell))
				.trim()
				.to_string();
			Self::write(world, cell, &address, style, &value);
			any = true;
		}
		Ok(any)
	}

	/// Writes `value` as `cell`'s only content, as a person typing it would:
	/// led by `=` its formula in an `f`, with no value until the book
	/// recalculates on open, a number in a `v`, anything else an inline
	/// string, a gap becoming the `c` it stands for.
	fn write(
		world: &mut World,
		cell: Entity,
		address: &CellAddress,
		style: Option<u32>,
		value: &str,
	) {
		let element = |entity: Entity| {
			world
				.entity(entity)
				.get::<OoxmlNode>()
				.and_then(OoxmlNode::element)
				.cloned()
		};
		let named = match element(cell) {
			Some(named) => named,
			None => {
				let row = world
					.entity(cell)
					.get::<ChildOf>()
					.and_then(|parent| element(parent.parent()))
					.unwrap_or_else(|| {
						OoxmlElement::new("row", Some(Ns::SPREADSHEET.into()))
					});
				let mut named = row.sibling(Ns::SPREADSHEET, "c");
				named.set_attribute(OoxmlAttribute::new(
					"r",
					None,
					address.a1(),
				));
				if let Some(style) = style.filter(|style| *style != 0) {
					named.set_attribute(OoxmlAttribute::new(
						"s",
						None,
						style.to_string(),
					));
				}
				named
			}
		};
		world.entity_mut(cell).despawn_children();
		let mut written = named.clone();
		let formula = value.strip_prefix('=').map(str::trim);
		let is_number = Self::is_number(value);
		match formula.is_some() || is_number {
			true => {
				written.remove_attribute(None, "t");
			}
			false => written.set_attribute(OoxmlAttribute::new(
				"t",
				None,
				"inlineStr",
			)),
		}
		world.entity_mut(cell).insert(OoxmlNode::Element(written));
		if let Some(formula) = formula {
			world.entity_mut(cell).insert(CellFormula(formula.into()));
			world.spawn((
				OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "f")),
				ChildOf(cell),
				children![Value::str(formula)],
			));
			return;
		}
		world.entity_mut(cell).remove::<CellFormula>();
		match is_number {
			true => {
				world.spawn((
					OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "v")),
					ChildOf(cell),
					children![Value::str(value)],
				));
			}
			false => {
				let mut text = named.sibling(Ns::SPREADSHEET, "t");
				text.set_attribute(OoxmlAttribute::new(
					"xml:space",
					Some(Ns::XML.into()),
					"preserve",
				));
				world.spawn((
					OoxmlNode::Element(named.sibling(Ns::SPREADSHEET, "is")),
					ChildOf(cell),
					children![(OoxmlNode::Element(text), children![
						Value::str(value)
					])],
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
		let part = OoxmlNode::spawn_part(&mut world, path.clone(), &nodes)?;
		let (book, calculation, after) =
			world.with_state::<OoxmlQuery, _>(|tree| {
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
		let full = OoxmlAttribute::new("fullCalcOnLoad", None, "1");
		match calculation {
			Some(calculation) => {
				if let Some(mut node) =
					world.entity_mut(calculation).get_mut::<OoxmlNode>()
					&& let Some(element) = node.element_mut()
				{
					element.set_attribute(full);
				}
			}
			None => {
				let named = world
					.with_state::<OoxmlQuery, _>(|query| {
						query.element(book).cloned()
					})
					.ok_or_else(|| bevyhow!("the workbook part has no root"))?;
				let mut element = named.sibling(Ns::SPREADSHEET, "calcPr");
				element.set_attribute(full);
				let calculation = world.spawn(OoxmlNode::Element(element)).id();
				world.entity_mut(book).insert_child(
					after.map_or(0, |index| index + 1),
					calculation,
				);
			}
		}
		let text =
			world.with_state::<OoxmlWriter, _>(|writer| writer.write(part));
		file.set_data(&path, text.into_bytes())
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	fn parse(bytes: &MediaBytes) -> (World, Entity) {
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

	fn write(
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
		part(bytes, "word/document.xml")
	}

	fn cells(world: &mut World, root: Entity) -> Vec<String> {
		CellText::listing(world, root)
			.iter()
			.map(|cell| cell.text.to_string())
			.collect()
	}

	/// Runs one edit on `world`'s document, its writes applied after.
	fn edit<O>(
		world: &mut World,
		func: impl FnOnce(&mut DocumentTreeEdit) -> O,
	) -> O {
		world.with_state::<DocumentTreeEdit, _>(|mut edit| func(&mut edit))
	}

	#[beet_core::test]
	fn untouched_files_write_back_render_identical() {
		for bytes in [super::super::word::test::form(), workbook()] {
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
	fn part(bytes: &MediaBytes, path: &str) -> String {
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
				workbook(),
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

	#[beet_core::test]
	fn fills_a_word_form() {
		let (mut world, root) = parse(&super::super::word::test::form());
		let cell = |text| CellAddress::parse(text).unwrap();
		edit(&mut world, |edit| {
			edit.set_cell_text(
				root,
				&cell("t1r1c2"),
				"Acme Stalls\nSecond line",
			)
		})
		.unwrap();
		edit(&mut world, |edit| {
			edit.append_cell_text(root, &cell("t1r1c1"), "appended")
		})
		.unwrap();
		edit(&mut world, |edit| edit.check_labeled_boxes(root, "Surveys"))
			.unwrap()
			.xpect_eq(1);
		edit(&mut world, |edit| edit.check_labeled_boxes(root, "Nothing"))
			.unwrap()
			.xpect_eq(0);
		edit(&mut world, |edit| {
			edit.remove_paragraphs(root, "Please delete this sentence")
		})
		.unwrap()
		.xpect_eq(1);
		edit(&mut world, |edit| {
			edit.replace_text(root, "(Insert name)", "Acme")
		})
		.unwrap()
		.xpect_eq(1);
		edit(&mut world, |edit| {
			edit.set_cell_text(root, &cell("t9r1c1"), "x")
		})
		.xpect_err();
		edit(&mut world, |edit| edit.remove_paragraphs(root, " ")).xpect_err();

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

	/// The element `tag` whose text holds `words`.
	fn find(world: &mut World, tag: &str, words: &str) -> Entity {
		let tagged = world
			.query::<(Entity, &Element)>()
			.iter(world)
			.filter(|(_, element)| element.tag() == tag)
			.map(|(entity, _)| entity)
			.collect::<Vec<_>>();
		world.with_state::<DocumentText, _>(|text| {
			tagged
				.into_iter()
				.find(|entity| text.text(*entity).contains(words))
				.unwrap()
		})
	}

	/// A new element `tag` with `attributes`, holding `text`, as an edit
	/// adds it, the last child of `parent`.
	fn add(
		world: &mut World,
		parent: Entity,
		tag: &str,
		attributes: &[(&str, &str)],
		text: &str,
	) -> Entity {
		let entity = world
			.spawn((Element::new(tag), ChildOf(parent), children![Value::str(
				text
			)]))
			.id();
		for (key, value) in attributes {
			world.spawn((
				AttributeOf::new(entity),
				Attribute::new(*key),
				Value::str(*value),
			));
		}
		entity
	}

	/// Parses `bytes`, runs `edit` on its tree, writes it back, and answers
	/// the written file with its markdown, read afresh.
	fn round_trip(
		bytes: MediaBytes,
		edit: impl FnOnce(&mut World, Entity),
	) -> (MediaBytes, String) {
		let (mut world, root) = parse(&bytes);
		edit(&mut world, root);
		let written = write(&mut world, root, MediaType::Docx);
		let (mut world, root) = parse(&written);
		let markdown = super::super::word::test::markdown(&mut world, root);
		(written, markdown)
	}

	/// A new heading takes the properties of a heading of its level, and
	/// with none its `Heading<n>` style.
	#[beet_core::test]
	fn writes_new_headings() {
		let (written, markdown) = round_trip(
			OoxmlFile::word(
				"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/><w:keepNext/></w:pPr><w:r><w:t>First</w:t></w:r></w:p>\
				 <w:p><w:r><w:t>Body</w:t></w:r></w:p>",
			)
			.unwrap(),
			|world, _| {
				let body = find(world, "p", "Body");
				let parent = world.entity(body).get::<ChildOf>().unwrap().parent();
				add(world, parent, "h2", &[], "Second");
			},
		);
		markdown.xpect_contains("## First\n\nBody\n\n## Second");
		document(&written).xpect_contains(
			"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/><w:keepNext/></w:pPr><w:r><w:t xml:space=\"preserve\">Second</w:t></w:r></w:p>",
		);

		let (written, markdown) = round_trip(
			OoxmlFile::word("<w:p><w:r><w:t>Body</w:t></w:r></w:p>").unwrap(),
			|world, _| {
				let body = find(world, "p", "Body");
				let parent =
					world.entity(body).get::<ChildOf>().unwrap().parent();
				add(world, parent, "h2", &[], "Second");
			},
		);
		markdown.xpect_contains("Body\n\n## Second");
		document(&written).xpect_contains(
			"<w:p><w:pPr><w:pStyle w:val=\"Heading2\"/></w:pPr><w:r><w:t xml:space=\"preserve\">Second</w:t></w:r></w:p>",
		);
	}

	/// A new list item joins its list by its neighbour's numbering.
	#[beet_core::test]
	fn joins_new_items_to_their_list() {
		round_trip(
			OoxmlFile::word(
				"<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>",
			)
			.unwrap(),
			|world, _| {
				let item = find(world, "li", "one");
				let list = world.entity(item).get::<ChildOf>().unwrap().parent();
				add(world, list, "li", &[], "two");
			},
		)
		.1
		.xpect_contains("- one\n- two\n");
	}

	/// New words inside new formatting elements are written with the run
	/// properties they say, each once and in schema order.
	#[beet_core::test]
	fn writes_new_inline_formatting() {
		let (written, markdown) = round_trip(
			OoxmlFile::word("<w:p><w:r><w:t>Plain</w:t></w:r></w:p>").unwrap(),
			|world, _| {
				let paragraph = find(world, "p", "Plain");
				for (tag, words) in
					[("strong", "bold"), ("em", "slanted"), ("mark", "lit")]
				{
					world.spawn((ChildOf(paragraph), Value::str(" ")));
					add(world, paragraph, tag, &[], words);
				}
				world.spawn((ChildOf(paragraph), Value::str(" ")));
				let strong = add(world, paragraph, "strong", &[], "");
				let em = add(world, strong, "em", &[], "");
				add(world, em, "b", &[], "both");
			},
		);
		markdown.xpect_contains(
			"Plain **bold** *slanted* <mark>lit</mark> ***both***",
		);
		document(&written)
			.xpect_contains("<w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">bold</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:i/></w:rPr><w:t xml:space=\"preserve\">slanted</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t xml:space=\"preserve\">lit</w:t></w:r>")
			.xpect_contains("<w:r><w:rPr><w:b/><w:i/></w:rPr><w:t xml:space=\"preserve\">both</w:t></w:r>");
	}

	/// New links in the main document and a header are hyperlinks of their
	/// part's relationships, one shared by two links to one address, in the
	/// look of the file's link; a changed link points at its new address.
	#[beet_core::test]
	fn writes_links_through_relationships() {
		let (written, markdown) = round_trip(
			OoxmlFile::word_with(
				"<w:p><w:hyperlink r:id=\"rIdOld\"><w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr><w:t>old</w:t></w:r></w:hyperlink></w:p>\
				 <w:p><w:r><w:t>See</w:t></w:r></w:p>",
				WordParts {
					header: Some("<w:p><w:r><w:t>Top</w:t></w:r></w:p>"),
					links: &[("rIdOld", "https://old.example/")],
					..default()
				},
			)
			.unwrap(),
			|world, _| {
				let see = find(world, "p", "See");
				for words in ["one", "two"] {
					world.spawn((ChildOf(see), Value::str(" ")));
					add(world, see, "a", &[("href", "https://a.example/")], words);
				}
				let top = find(world, "p", "Top");
				world.spawn((ChildOf(top), Value::str(" ")));
				add(world, top, "a", &[("href", "https://b.example/")], "head");
				let old = find(world, "a", "old");
				let href = world.with_state::<ElementQuery, _>(|elements| {
					elements
						.get(old)
						.unwrap()
						.attribute_with_entity("href")
						.unwrap()
						.0
				});
				world
					.entity_mut(href)
					.insert(Value::str("https://new.example/"));
			},
		);
		markdown
			.xpect_contains("Top [head](https://b.example/)")
			.xpect_contains("[old](https://new.example/)")
			.xpect_contains(
				"See [one](https://a.example/) [two](https://a.example/)",
			);
		let file = OoxmlFile::open(&written).unwrap();
		let main = file.part_at("word/document.xml").unwrap();
		let mut targets =
			file.hyperlinks(&main).into_values().collect::<Vec<_>>();
		targets.sort();
		// the old address kept, since another link may share it
		targets.xpect_eq(vec![
			SmolStr::new("https://a.example/"),
			"https://new.example/".into(),
			"https://old.example/".into(),
		]);
		let link_run = "<w:r><w:rPr><w:rStyle w:val=\"Hyperlink\"/></w:rPr><w:t xml:space=\"preserve\">";
		document(&written)
			.xpect_contains(format!("{link_run}one</w:t>"))
			.xpect_contains(format!("{link_run}two</w:t>"));
		let header = file
			.parts()
			.into_iter()
			.find(|part| {
				matches!(part, ooxmlsdk::parts::PartRef::HeaderPart(_))
			})
			.unwrap();
		let header_path = file.path(&header).unwrap();
		// the header's root binds no `r:`, so the link binds it
		part(&written, &header_path).xpect_contains(format!(
			"<w:hyperlink xmlns:r=\"{}\" r:id=\"",
			OoxmlNamespace::RELATIONSHIPS
		));
		file.hyperlinks(&header)
			.into_values()
			.collect::<Vec<_>>()
			.xpect_eq(vec![SmolStr::new("https://b.example/")]);
	}

	/// A new table and a new link to a place in the file are written as
	/// their text, and named as dropped.
	#[beet_core::test]
	fn writes_what_does_not_reverse_as_text() {
		let bytes =
			OoxmlFile::word("<w:p><w:r><w:t>Body</w:t></w:r></w:p>").unwrap();
		let edit = |world: &mut World, _: Entity| {
			let body = find(world, "p", "Body");
			add(world, body, "a", &[("href", "#place")], " here");
			let parent = world.entity(body).get::<ChildOf>().unwrap().parent();
			let table = add(world, parent, "table", &[], "");
			let row = add(world, table, "tr", &[], "");
			add(world, row, "td", &[], "cell");
		};
		let (mut world, root) = parse(&bytes);
		edit(&mut world, root);
		super::WordWriter::dropped(&mut world, root)
			.xpect_eq(vec!["a href=\"#..\"", "table"]);
		// a cell added to a table the file holds
		let (mut world, root) = parse(&super::super::word::test::form());
		let row = find(&mut world, "tr", "Name");
		add(&mut world, row, "td", &[], "extra");
		super::WordWriter::dropped(&mut world, root).xpect_eq(vec!["td"]);
		let (written, markdown) = round_trip(bytes, edit);
		markdown
			.xpect_contains("Body here\n\ncell")
			.xnot()
			.xpect_contains("[here]");
		document(&written).xnot().xpect_contains("w:tbl");
	}

	/// A protected sheet: `A1` a locked label, `D3:F3` an unlocked merged
	/// field holding a prompt, `D4` an unlocked formula, and column `G`
	/// unlocked by its column style where no cell is written.
	fn workbook() -> MediaBytes { sheet("<sheetProtection sheet=\"1\"/>") }

	/// [`workbook`]'s sheet with `protection` after its data, its cells
	/// locked only when that protects it.
	fn sheet(protection: &str) -> MediaBytes {
		OoxmlFile::workbook(
			"<xf/><xf><protection locked=\"0\"/></xf>",
			&[(
				"Start Here",
				&format!("<cols><col min=\"7\" max=\"7\" style=\"1\"/></cols><sheetData>\
				 <row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Business name</t></is></c></row>\
				 <row r=\"3\"><c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t>Enter it here</t></is></c>\
				 <c r=\"E3\" s=\"1\"/><c r=\"F3\" s=\"1\"/><c r=\"H3\"/></row>\
				 <row r=\"4\"><c r=\"D4\" s=\"1\"><f>1+1</f><v>2</v></c></row>\
				 </sheetData>{protection}<mergeCells count=\"1\"><mergeCell ref=\"D3:F3\"/></mergeCells>"),
			)],
		)
		.unwrap()
	}

	/// An unprotected sheet locks nothing, whatever its formats say.
	#[beet_core::test]
	fn unprotected_sheets_lock_nothing() {
		let (mut world, root) = parse(&sheet(""));
		CellText::listing(&mut world, root)
			.into_iter()
			.map(|cell| cell.address.to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"Start Here!A1".to_string(),
				"Start Here!A3".into(),
				"Start Here!B3".into(),
				"Start Here!C3".into(),
				"Start Here!D3".into(),
				"Start Here!G3".into(),
				"Start Here!H3".into(),
				"Start Here!A4".into(),
				"Start Here!B4".into(),
				"Start Here!C4".into(),
				"Start Here!D4".into(),
			]);
		for cell in ["Start Here!A1", "Start Here!H3"] {
			edit(&mut world, |edit| {
				edit.set_cell_text(
					root,
					&CellAddress::parse(cell).unwrap(),
					"x",
				)
			})
			.unwrap();
		}
	}

	/// A protected sheet's locked cells say so in HTML, and its markdown,
	/// which has no say over editing, reads as an unprotected one's.
	#[beet_core::test]
	fn renders_locked_cells() {
		fn render(
			bytes: &MediaBytes,
			mut renderer: impl DocRenderer,
		) -> String {
			let (mut world, root) = parse(bytes);
			renderer
				.render(&mut RenderContext::new(
					&mut world,
					root,
					&RequestParts::default(),
				))
				.unwrap()
				.to_string()
		}
		render(&workbook(), HtmlRenderer::new())
			.xpect_starts_with("<table><caption>Start Here</caption>")
			.xpect_contains("<td contenteditable=\"false\">Business name</td>")
			.xpect_contains("<td colspan=\"3\">Enter it here</td><td></td><td contenteditable=\"false\"></td>")
			.xpect_contains("<td>2</td>");
		render(&workbook(), MarkdownRenderer::new())
			.xpect_eq(render(&sheet(""), MarkdownRenderer::new()));
	}

	#[beet_core::test]
	fn lists_and_sets_unlocked_cells() {
		let (mut world, root) = parse(&workbook());
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| Start Here!D3 | Enter it here |".to_string(),
				"| Start Here!G3 | (empty) |".into(),
				"| Start Here!D4 | =1+1 |".into(),
			]);
		let set = |world: &mut World, cell: &str, text: &str| {
			edit(world, |edit| {
				edit.set_cell_text(
					root,
					&CellAddress::parse(cell).unwrap(),
					text,
				)
			})
		};
		// a merged range lists once, and any slot it covers writes its first
		let written = set(&mut world, "Start Here!E3", "Acme Stalls").unwrap();
		world
			.entity(written)
			.get::<CellAddress>()
			.unwrap()
			.to_string()
			.xpect_eq("Start Here!D3");
		set(&mut world, "Start Here!G3", "120.5").unwrap();
		set(&mut world, "Start Here!A1", "x")
			.unwrap_err()
			.to_string()
			.xpect_contains("is locked");
		// a formula replaces a formula, computed when the book is next opened
		set(&mut world, "Start Here!D4", "=G3*2").unwrap();
		set(&mut world, "Nowhere!A1", "x").xpect_err();

		let bytes = write(&mut world, root, MediaType::Xlsx);
		part(&bytes, "xl/worksheets/sheet1.xml")
			.xpect_contains("<c r=\"D3\" s=\"1\" t=\"inlineStr\"><is><t xml:space=\"preserve\">Acme Stalls</t></is></c>")
			// the gap's cell, created with its column's style
			.xpect_contains("<c r=\"G3\" s=\"1\"><v>120.5</v></c><c r=\"H3\"/>")
			.xpect_contains("<c r=\"D4\" s=\"1\"><f>G3*2</f></c>");
		part(&bytes, "xl/workbook.xml")
			.xpect_contains("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/>");
		let (mut world, root) = parse(&bytes);
		cells(&mut world, root).xpect_eq(vec![
			"Acme Stalls".to_string(),
			"120.5".into(),
			"=G3*2".into(),
		]);
	}
}
