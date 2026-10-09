//! A Word file read as the one tree: what each WordprocessingML node means to
//! a reader, as HTML where it maps cleanly and components where it does not.
use super::ooxml_query::*;
use crate::prelude::*;
use beet_core::prelude::*;
use ooxmlsdk::parts::PartRef;

type Ns = OoxmlNamespace;

/// A Word paragraph's style, its `w:pStyle`, ie `Heading1`.
#[derive(Debug, Clone, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct ParagraphStyle(pub SmolStr);

/// A numbered paragraph's list level from 0, its `w:numPr/w:ilvl`: the
/// paragraph is an `<li>` in a projection-only `<ul>` per level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deref, Reflect, Component)]
#[reflect(Component)]
pub struct ListLevel(pub u8);

/// A Word run's whole look, its `w:rPr` as a reader sees it: what a form
/// signals by look, ie a highlighted placeholder or a red instruction.
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Component)]
#[reflect(Component, Default)]
pub struct RunLook {
	/// `w:b`.
	pub bold: bool,
	/// `w:i`.
	pub italic: bool,
	/// The highlight colour by name, ie `yellow`, `w:highlight`.
	pub highlight: Option<SmolStr>,
	/// The shading fill, `RRGGBB`, `w:shd`.
	pub shading: Option<SmolStr>,
	/// The text colour, `RRGGBB`, absent for automatic or black, `w:color`.
	pub colour: Option<SmolStr>,
}

impl RunLook {
	/// The elements the look projects to, outermost first: its highlight,
	/// else its shading, as a `<mark>`, else its colour as a styled
	/// `<span>`, then `<strong>`, then `<em>`.
	pub(crate) fn elements(&self) -> Vec<Tagged> {
		let wrap = match (&self.highlight, &self.shading, &self.colour) {
			(Some(highlight), ..) if highlight == "yellow" => {
				Some(Tagged::new("mark"))
			}
			(Some(highlight), ..) => Some(
				Tagged::new("mark")
					.with("style", format!("background: {highlight}")),
			),
			(None, Some(fill), _) => Some(
				Tagged::new("mark")
					.with("style", format!("background: #{fill}")),
			),
			(None, None, Some(colour)) => Some(
				Tagged::new("span").with("style", format!("color: #{colour}")),
			),
			(None, None, None) => None,
		};
		wrap.into_iter()
			.chain(self.bold.then(|| Tagged::new("strong")))
			.chain(self.italic.then(|| Tagged::new("em")))
			.collect()
	}

	/// Automatic or white, which is no look at all.
	fn is_plain(colour: &str) -> bool {
		matches!(colour.to_ascii_lowercase().as_str(), "auto" | "ffffff")
	}
}

/// Reads one Word part's source tree, the main document, a header or a
/// footer or the notes, into the one tree.
pub(crate) struct WordProjection {
	/// The part's hyperlink targets by relationship id.
	pub links: HashMap<SmolStr, SmolStr>,
	/// The part's image paths by relationship id.
	pub images: HashMap<SmolStr, SmolStr>,
}

impl WordProjection {
	/// Projects the source tree under `part`, then groups its numbered
	/// paragraphs into lists.
	pub fn project(&self, world: &mut World, part: Entity) {
		let changes =
			world.with_state::<OoxmlQuery, _>(|tree| self.read(&tree, part));
		Projected::apply(world, changes);
		ListLevel::wrap(world, part);
	}

	fn read(&self, tree: &OoxmlQuery, part: Entity) -> Vec<Projected> {
		let mut changes = Vec::new();
		// text a checkbox's glyph is written in, which its `<input>` shows
		let mut glyphs = HashSet::<Entity>::default();
		for entity in tree.descendants(part) {
			let Some(element) = tree.element(entity) else {
				if tree.is_text(entity)
					&& (glyphs.contains(&entity)
						|| !Self::is_read(tree, entity))
				{
					changes.push(Projected::Hide(entity));
				}
				continue;
			};
			let Some(namespace) = element.namespace.as_deref() else {
				continue;
			};
			let in_run = tree
				.parent(entity)
				.is_some_and(|parent| tree.is(parent, Ns::WORD, "r"));
			let tag = |tag: &str| Projected::Tag(entity, Tagged::new(tag));
			match (namespace, element.local_name()) {
				(Ns::WORD, "p") => self.paragraph(tree, entity, &mut changes),
				(Ns::WORD, "r") => self.run(tree, entity, &mut changes),
				(Ns::WORD, "tbl") => changes.push(tag("table")),
				(Ns::WORD, "tr") => changes.push(tag("tr")),
				(Ns::WORD, "tc") => changes.push(self.cell(tree, entity)),
				(Ns::WORD, "hyperlink") => {
					if let Some(link) = self.link(tree, entity) {
						changes.push(link);
					}
				}
				// a revision of content, never of a properties element
				(Ns::WORD, "ins" | "moveTo")
					if !Self::in_properties(tree, entity) =>
				{
					changes.push(tag("ins"))
				}
				(Ns::WORD, "del" | "moveFrom")
					if !Self::in_properties(tree, entity) =>
				{
					changes.push(tag("del"))
				}
				(Ns::WORD, "sdt") => {
					if let Some(checkbox) =
						self.checkbox(tree, entity, &mut glyphs)
					{
						changes.extend(checkbox);
					}
				}
				(Ns::WORD, "tab") if in_run => {
					changes.push(Projected::Show(entity, " ".into()))
				}
				(Ns::WORD, "noBreakHyphen") if in_run => {
					changes.push(Projected::Show(entity, "-".into()))
				}
				(Ns::WORD, "sym") if in_run => {
					if let Some(symbol) = tree
						.attribute(entity, Some(Ns::WORD), "char")
						.and_then(|code| u32::from_str_radix(code, 16).ok())
						.and_then(char::from_u32)
					{
						changes.push(Projected::Show(
							entity,
							symbol.to_string().into(),
						))
					}
				}
				(Ns::WORD, "br" | "cr") if in_run => changes.push(tag("br")),
				(Ns::WORD, "txbxContent") => changes.push(tag("aside")),
				(Ns::WORD, "hdr") => changes.push(tag("header")),
				(Ns::WORD, "ftr") => changes.push(tag("footer")),
				(Ns::WORD, "footnotes" | "endnotes") => {
					changes.push(tag("aside"))
				}
				// what an older reader shows instead of the choice above it
				(Ns::COMPATIBILITY, "Fallback") => {
					changes.push(tag("template"))
				}
				(Ns::DRAWING, "blip") => {
					if let Some(image) = self.image(tree, entity) {
						changes.push(image);
					}
				}
				_ => {}
			}
		}
		changes
	}

	/// Whether a text node is words a reader reads: a `w:t`'s, or a
	/// `w:delText`'s, which a `<del>` shows struck. Any other, ie a field's
	/// instruction or the whitespace between elements, only the writer
	/// reads.
	fn is_read(tree: &OoxmlQuery, text: Entity) -> bool {
		tree.parent(text)
			.and_then(|parent| tree.element(parent))
			.is_some_and(|parent| {
				matches!(parent.local_name(), "t" | "delText")
			})
	}

	/// Whether `entity` sits in a properties element, ie a `w:ins` marking
	/// a paragraph mark inserted.
	fn in_properties(tree: &OoxmlQuery, entity: Entity) -> bool {
		tree.parent(entity)
			.and_then(|parent| tree.element(parent))
			.is_some_and(|parent| parent.local_name().ends_with("Pr"))
	}

	/// A paragraph: a heading by its style, `Title` the first, a list item
	/// when numbered, else a `<p>`.
	fn paragraph(
		&self,
		tree: &OoxmlQuery,
		entity: Entity,
		changes: &mut Vec<Projected>,
	) {
		let style = tree
			.property(entity, "pPr", Ns::WORD, "pStyle", "val")
			.map(SmolStr::new);
		let level = tree
			.child(entity, Ns::WORD, "pPr")
			.and_then(|properties| tree.child(properties, Ns::WORD, "numPr"))
			.map(|numbering| {
				tree.child(numbering, Ns::WORD, "ilvl")
					.and_then(|level| {
						tree.attribute(level, Some(Ns::WORD), "val")
					})
					.and_then(|level| level.parse::<u8>().ok())
					.unwrap_or(0)
			});
		let heading = match style.as_deref() {
			Some("Title") => Some(1),
			Some(style) => style
				.strip_prefix("Heading")
				.and_then(|level| level.parse::<u8>().ok())
				.filter(|level| (1..=6).contains(level)),
			None => None,
		};
		let tag = match (heading, level) {
			(Some(heading), _) => format!("h{heading}"),
			(None, Some(level)) => {
				changes.push(Projected::insert(entity, ListLevel(level)));
				"li".into()
			}
			(None, None) => "p".into(),
		};
		changes.push(Projected::Tag(entity, Tagged::new(tag)));
		if let Some(style) = style {
			changes.push(Projected::insert(entity, ParagraphStyle(style)));
		}
	}

	/// A run: its look the run's element, its inner looks projection-only
	/// wrappers around everything after its properties.
	fn run(
		&self,
		tree: &OoxmlQuery,
		entity: Entity,
		changes: &mut Vec<Projected>,
	) {
		let property = |local: &str, attribute: &str| {
			tree.property(entity, "rPr", Ns::WORD, local, attribute)
				.map(SmolStr::new)
		};
		let look = RunLook {
			bold: tree.toggle(entity, "rPr", Ns::WORD, "b"),
			italic: tree.toggle(entity, "rPr", Ns::WORD, "i"),
			highlight: property("highlight", "val")
				.filter(|highlight| highlight != "none"),
			shading: property("shd", "fill")
				.filter(|fill| !RunLook::is_plain(fill)),
			colour: property("color", "val").filter(|colour| {
				!RunLook::is_plain(colour) && colour != "000000"
			}),
		};
		let mut elements = look.elements().into_iter();
		if let Some(outer) = elements.next() {
			changes.push(Projected::Tag(entity, outer));
			let wrappers = elements.collect::<Vec<_>>();
			if !wrappers.is_empty() {
				changes.push(Projected::Wrap {
					entity,
					wrappers,
					content: tree
						.children(entity)
						.into_iter()
						.filter(|child| !tree.is(*child, Ns::WORD, "rPr"))
						.collect(),
				});
			}
		}
		if look != RunLook::default() {
			changes.push(Projected::insert(entity, look));
		}
	}

	/// A table cell, its span and its shading kept as `colspan` and a
	/// background.
	fn cell(&self, tree: &OoxmlQuery, entity: Entity) -> Projected {
		let mut cell = Tagged::new("td");
		if let Some(span) = tree
			.property(entity, "tcPr", Ns::WORD, "gridSpan", "val")
			.filter(|span| *span != "1")
		{
			cell = cell.with("colspan", span);
		}
		if let Some(fill) = tree
			.property(entity, "tcPr", Ns::WORD, "shd", "fill")
			.filter(|fill| !RunLook::is_plain(fill))
		{
			cell = cell.with("style", format!("background: #{fill}"));
		}
		Projected::Tag(entity, cell)
	}

	/// A hyperlink to its relationship's target. A link to a bookmark in
	/// the file, ie a table of contents entry, stays its text, since no
	/// bookmark is an element a reader could follow it to.
	fn link(&self, tree: &OoxmlQuery, entity: Entity) -> Option<Projected> {
		let href = tree
			.attribute(entity, Some(Ns::RELATIONSHIPS), "id")
			.and_then(|id| self.links.get(id))?;
		Projected::Tag(entity, Tagged::new("a").with("href", href.as_str()))
			.xsome()
	}

	/// A checkbox content control as the `<input>` it is, its glyph text
	/// kept for the writer since the input shows it.
	fn checkbox(
		&self,
		tree: &OoxmlQuery,
		entity: Entity,
		glyphs: &mut HashSet<Entity>,
	) -> Option<Vec<Projected>> {
		let properties = tree.child(entity, Ns::WORD, "sdtPr")?;
		let checkbox =
			tree.descendant(properties, Ns::WORD_2010, "checkbox")?;
		let checked = tree
			.child(checkbox, Ns::WORD_2010, "checked")
			.and_then(|checked| {
				tree.attribute(checked, Some(Ns::WORD_2010), "val")
			})
			.is_some_and(|value| matches!(value, "1" | "true"));
		if let Some(content) = tree.child(entity, Ns::WORD, "sdtContent") {
			glyphs.extend(
				tree.descendants(content)
					.into_iter()
					.filter(|descendant| tree.is_text(*descendant)),
			);
		}
		let mut input = Tagged::new("input").with("type", "checkbox");
		if checked {
			input = input.flag("checked");
		}
		vec![
			Projected::Tag(entity, input),
			Projected::insert(entity, Value::Bool(checked)),
		]
		.xsome()
	}

	/// A picture as an `<img>` of its image part, named by its drawing's
	/// name and description.
	fn image(&self, tree: &OoxmlQuery, entity: Entity) -> Option<Projected> {
		let source = tree
			.attribute(entity, Some(Ns::RELATIONSHIPS), "embed")
			.and_then(|id| self.images.get(id))?;
		let described = ["inline", "anchor"]
			.iter()
			.find_map(|local| tree.ancestor(entity, Ns::WORD_DRAWING, local))
			.and_then(|drawing| tree.child(drawing, Ns::WORD_DRAWING, "docPr"))
			.map(|picture| {
				["name", "descr"]
					.iter()
					.filter_map(|key| tree.attribute(picture, None, key))
					.filter(|part| !part.is_empty())
					.collect::<Vec<_>>()
					.join(", ")
			})
			.unwrap_or_default();
		Projected::Tag(
			entity,
			Tagged::new("img")
				.with("src", source.as_str())
				.with("alt", described),
		)
		.xsome()
	}
}

impl WordProjection {
	/// Captions every numbered table under `part` with the number its cells
	/// are addressed by, as a comment, ie `<!-- t3 -->`: a projection-only
	/// `<caption>` a reader sees above the table and a writer never writes.
	fn caption_tables(world: &mut World, part: Entity) {
		let numbered = world.with_state::<(
			Query<(Entity, &Element)>,
			Query<&Children>,
			Query<&TableCellAddress>,
		), _>(|(elements, children, addresses)| {
			children
				.iter_descendants_depth_first(part)
				.filter(|entity| {
					elements
						.get(*entity)
						.is_ok_and(|(_, element)| element.tag() == "table")
				})
				.filter_map(|table| {
					children
						.iter_descendants_depth_first(table)
						.find_map(|cell| addresses.get(cell).ok())
						.map(|address| (table, address.table))
				})
				.collect::<Vec<_>>()
		});
		for (table, number) in numbered {
			let caption = world
				.spawn((Element::new("caption"), children![Comment::new(
					format!(" t{number} ")
				)]))
				.id();
			world.entity_mut(table).insert_child(0, caption);
		}
	}
}

impl ListLevel {
	/// Groups every run of numbered paragraphs under `part` into
	/// projection-only `<ul>`s, one per level, a deeper level nested in the
	/// one above it. Any other sibling ends the lists open around it.
	pub(crate) fn wrap(world: &mut World, part: Entity) {
		let containers =
			world
				.with_state::<(Query<&Children>, Query<&ChildOf, With<ListLevel>>), _>(
					|(children, items)| {
						let mut containers = Vec::<Entity>::new();
						for item in children
							.iter_descendants_depth_first(part)
							.filter_map(|entity| items.get(entity).ok())
						{
							if !containers.contains(&item.parent()) {
								containers.push(item.parent());
							}
						}
						containers
					},
				);
		for container in containers {
			Self::wrap_container(world, container);
		}
	}

	fn wrap_container(world: &mut World, container: Entity) {
		let children = world
			.entity(container)
			.get::<Children>()
			.map(|children| children.to_vec())
			.unwrap_or_default();
		// the open lists, outermost first
		let mut lists = Vec::<Entity>::new();
		for child in children {
			let Some(level) = world.entity(child).get::<ListLevel>() else {
				lists.clear();
				continue;
			};
			let depth = level.0 as usize + 1;
			lists.truncate(depth);
			while lists.len() < depth {
				let list = world.spawn(Element::new("ul")).id();
				match lists.last() {
					Some(parent) => {
						world.entity_mut(*parent).add_child(list);
					}
					None => {
						let index = world
							.entity(container)
							.get::<Children>()
							.and_then(|children| {
								children
									.iter()
									.position(|entity| entity == child)
							})
							.unwrap_or_default();
						world.entity_mut(container).insert_child(index, list);
					}
				}
				lists.push(list);
			}
			world.entity_mut(*lists.last().unwrap()).add_child(child);
		}
	}
}

impl WordProjection {
	/// Reads a Word package under `root`: its headers, its main document,
	/// its footers and its notes, each a part read and projected, and the
	/// main document's tables numbered.
	pub(crate) fn build(
		world: &mut World,
		root: Entity,
		package: &OoxmlFile,
	) -> Result {
		let main = package
			.parts()
			.into_iter()
			.find(|part| matches!(part, PartRef::MainDocumentPart(_)))
			.ok_or_else(|| {
				bevyhow!("the Word file has no main document part")
			})?;
		let related = package
			.related(&main)
			.into_iter()
			.map(|(_, part)| part)
			.collect::<Vec<_>>();
		let of_kind = |is: fn(&PartRef) -> bool| {
			related.iter().filter(move |part| is(part)).cloned()
		};
		let parts = of_kind(|part| matches!(part, PartRef::HeaderPart(_)))
			.chain([main.clone()])
			.chain(of_kind(|part| matches!(part, PartRef::FooterPart(_))))
			.chain(of_kind(|part| matches!(part, PartRef::FootnotesPart(_))))
			.chain(of_kind(|part| matches!(part, PartRef::EndnotesPart(_))))
			.collect::<Vec<_>>();
		for part in parts {
			let entity = OoxmlParser::spawn_part(world, root, package, &part)?;
			Self {
				links: package.hyperlinks(&part),
				images: package.images(&part),
			}
			.project(world, entity);
			// the cells a fill addresses are the main document's alone
			if part == main {
				TableCellAddress::assign(world, entity);
				Self::caption_tables(world, entity);
			}
		}
		Ok(())
	}
}

#[cfg(test)]
pub(crate) mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// A Word file parsed into a world with the parse plugins.
	pub(crate) fn parse(bytes: MediaBytes) -> (World, Entity) {
		let mut world =
			(TemplatePlugin, DocumentPlugin, ParsePlugin).into_world();
		let root = world.spawn_empty().id();
		MediaParser::new()
			.parse(ParseContext::new(&mut world.entity_mut(root), &bytes))
			.unwrap();
		(world, root)
	}

	pub(crate) fn markdown(world: &mut World, root: Entity) -> String {
		MarkdownRenderer::new()
			.render(&mut RenderContext::new(
				world,
				root,
				&RequestParts::default(),
			))
			.unwrap()
			.to_string()
	}

	#[beet_core::test]
	fn reads_a_word_file_in_html_terms() {
		let (mut world, root) = parse(
			OoxmlFile::word(
				"<w:p><w:pPr><w:pStyle w:val=\"Heading1\"/></w:pPr><w:r><w:t>Cover</w:t></w:r></w:p>\
				 <w:p><w:r><w:t xml:space=\"preserve\">Where is it? </w:t></w:r>\
				 <w:r><w:rPr><w:color w:val=\"FF0000\"/></w:rPr><w:t>(Please delete</w:t></w:r>\
				 <w:r><w:rPr><w:color w:val=\"FF0000\"/></w:rPr><w:t xml:space=\"preserve\"> this)</w:t></w:r></w:p>\
				 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>\
				 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>nested</w:t></w:r></w:p>\
				 <w:tbl><w:tr><w:tc><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Name</w:t></w:r></w:p></w:tc>\
				 <w:tc><w:tcPr><w:gridSpan w:val=\"2\"/></w:tcPr><w:p><w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>a &lt; b</w:t></w:r></w:p></w:tc></w:tr></w:tbl>\
				 <w:p><w:r><w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">Date </w:t></w:r><w:r><w:t>due</w:t></w:r>\
				 <w:r><w:instrText xml:space=\"preserve\"> PAGE </w:instrText></w:r></w:p>",
			)
			.unwrap(),
		);
		markdown(&mut world, root)
			.xpect_contains("# Cover\n\nWhere is it? <span style=\"color: #FF0000\">(Please delete this)</span>\n")
			.xpect_contains("- one\n  - nested\n")
			.xpect_contains("<!-- t1 -->\n\n| **Name** | <mark>a < b</mark> |  |\n|---|---|---|\n")
			// edge whitespace sits outside the emphasis, a field's
			// instruction is no reader's
			.xpect_contains("**Date** due\n")
			.xnot()
			.xpect_contains("PAGE");
		world
			.query::<&RunLook>()
			.iter(&world)
			.any(|look| look.colour.as_deref() == Some("FF0000"))
			.xpect_true();
	}

	/// The cells a fill addresses, a checkbox read as its box.
	#[beet_core::test]
	fn lists_the_cells_of_a_form() {
		let (mut world, root) = parse(form());
		CellText::listing(&mut world, root)
			.iter()
			.map(ToString::to_string)
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				"| t1r1c1 | Name |".to_string(),
				"| t1r1c2 | (empty) |".into(),
				"| t1r2c1 | Describe it: (Please delete this sentence once completed) |".into(),
				"| t1r2c2 | [ ] Surveys |".into(),
			]);
	}

	/// A cell with a prompt and a checkbox, a red sentence and a highlighted
	/// placeholder, as the forms a builder fills carry them.
	pub(crate) fn form() -> MediaBytes {
		OoxmlFile::word(
			"<w:tbl><w:tr>\
			 <w:tc><w:p><w:r><w:rPr><w:sz w:val=\"20\"/></w:rPr><w:t>Name</w:t></w:r></w:p></w:tc>\
			 <w:tc><w:p><w:pPr><w:jc w:val=\"left\"/></w:pPr></w:p></w:tc>\
			 </w:tr><w:tr><w:tc>\
			 <w:p><w:r><w:t>Describe it: (Please delete this sentence once completed)</w:t></w:r></w:p>\
			 </w:tc><w:tc><w:p>\
			 <w:sdt><w:sdtPr><w14:checkbox><w14:checked w14:val=\"0\"/></w14:checkbox></w:sdtPr>\
			 <w:sdtContent><w:r><w:rPr><w:sz w:val=\"20\"/></w:rPr><w:t>\u{2610}</w:t></w:r></w:sdtContent></w:sdt>\
			 <w:r><w:t xml:space=\"preserve\"> Surveys</w:t></w:r></w:p></w:tc>\
			 </w:tr></w:tbl>\
			 <w:p><w:r><w:t xml:space=\"preserve\">The business is </w:t></w:r>\
			 <w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>(Insert</w:t></w:r>\
			 <w:r><w:rPr><w:highlight w:val=\"yellow\"/></w:rPr><w:t>\u{a0}name)</w:t></w:r></w:p>",
		)
		.unwrap()
	}
}
