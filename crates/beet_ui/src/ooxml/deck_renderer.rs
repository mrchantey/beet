//! [`DeckRenderer`], a tree written as a slide deck, by the
//! [module docs](super)' deck mapping.
use super::office_picture::*;
use super::office_style::*;
use super::xml_part::XmlPart;
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// Writes a tree as a slide deck: each `article data-card` a slide, or the
/// whole document one slide when it holds none, its front through the
/// layout it names and its content after its first `hr` its speaker notes.
/// See the [module docs](super) for the mapping and what a write cannot
/// say.
#[derive(Debug, Default, Clone)]
pub struct DeckRenderer;

/// A chain of probe elements, outermost first.
type Chain = Vec<ProbeTag>;

/// The layout a slide writes with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
	/// A centered title over its subtitle.
	Title,
	/// A title over one column.
	Content,
	/// A title over two or more columns.
	TwoContent,
}

/// One slide as gathered from the tree.
#[derive(Debug)]
struct SlideWrite {
	hidden: bool,
	layout: Layout,
	/// The title's paragraph, written in the title placeholder.
	title: Option<ParagraphWrite>,
	/// A title slide's subtitle paragraphs.
	subtitle: Vec<ParagraphWrite>,
	/// Each column's paragraphs and objects, one column when it has no title.
	columns: Vec<ColumnWrite>,
	notes: Vec<ParagraphWrite>,
}

/// One column of a slide: its text, then its pictures and tables in flow
/// order.
#[derive(Debug, Default)]
struct ColumnWrite {
	paragraphs: Vec<ParagraphWrite>,
	objects: Vec<ObjectWrite>,
}

/// A shape of its own below a column's text.
#[derive(Debug)]
enum ObjectWrite {
	Picture(Entity),
	Table(TableWrite),
}

/// What a paragraph is to a text frame.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParagraphKind {
	Plain,
	/// A heading below the title, written bold.
	Heading,
	/// A list item at `level`.
	Item {
		level: u8,
		ordered: bool,
	},
}

/// One paragraph of a text frame.
#[derive(Debug)]
struct ParagraphWrite {
	kind: ParagraphKind,
	/// The element its alignment resolves at.
	entity: Entity,
	/// The structure a reader reads it back into.
	chain: Chain,
	inline: Vec<InlineWrite>,
}

/// One piece of a paragraph.
#[derive(Debug)]
enum InlineWrite {
	Text {
		text: String,
		run: RunContext,
		/// The element its look resolves at, its text's parent.
		look: Entity,
	},
	Break,
}

/// Where a run sits among the inline elements around it.
#[derive(Debug, Clone, Default)]
struct RunContext {
	/// The structural inline elements a reader reads back, outermost first.
	tags: Vec<ProbeTag>,
	/// Every inline element around it, outermost first.
	ancestors: Vec<Entity>,
	link: Option<String>,
	monospace: bool,
	/// A superscript's or subscript's baseline shift in thousandths of a
	/// percent.
	baseline: Option<i32>,
}

/// A table as gathered.
#[derive(Debug)]
struct TableWrite {
	columns: u32,
	header: bool,
	rows: Vec<Vec<CellWrite>>,
}

/// One slot of a table row: a cell, or the part of a merge it covers.
#[derive(Debug)]
struct CellWrite {
	entity: Option<Entity>,
	span: UVec2,
	/// Covered by a merge from the left, from above.
	covered: (bool, bool),
	paragraphs: Vec<ParagraphWrite>,
}

/// The tree read for writing.
#[derive(SystemParam)]
struct DeckGather<'w, 's> {
	tree: RenderTreeQuery<'w, 's>,
	elements: ElementQuery<'w, 's>,
	values: Query<'w, 's, &'static Value, Without<Element>>,
	tags: Query<'w, 's, &'static Element>,
	grid: TableGrid<'w, 's>,
}

/// Everything a write resolves before it writes.
struct Resolved {
	styles: OfficeStyleSheet,
	computed: HashMap<Entity, ComputedStyle>,
	probes: HashMap<Chain, Vec<ComputedStyle>>,
	/// Each `img`, unsized.
	images: HashMap<Entity, OfficePicture>,
}

/// The write in progress: the package, and the counters its parts share.
struct DeckWrite<'a> {
	resolved: &'a Resolved,
	package: OpcPackage,
	images: u32,
}

impl DeckRenderer {
	/// The slide in px, 16:9 at 13.333 by 7.5 inches.
	const SLIDE: Vec2 = Vec2::new(1280., 720.);
	/// The margin around a slide's shapes in px.
	const MARGIN: f32 = 48.;
	/// The gap between a slide's shapes in px.
	const GAP: f32 = 16.;
	/// The title band of a content slide in px.
	const TITLE_HEIGHT: f32 = 104.;
	const PRESENTATION: &str = "ppt/presentation.xml";
	const MASTER: &str = "ppt/slideMasters/slideMaster1.xml";
	const NOTES_MASTER: &str = "ppt/notesMasters/notesMaster1.xml";
	const THEME: &str = "ppt/theme/theme1.xml";
	const NOTES_THEME: &str = "ppt/theme/theme2.xml";
	const PRES_PROPS: &str = "ppt/presProps.xml";
	const PRESENTATION_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml";
	const SLIDE_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slide+xml";
	const LAYOUT_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml";
	const MASTER_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml";
	const NOTES_SLIDE_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml";
	const NOTES_MASTER_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml";
	const THEME_TYPE: &str =
		"application/vnd.openxmlformats-officedocument.theme+xml";
	const PRES_PROPS_TYPE: &str = "application/vnd.openxmlformats-officedocument.presentationml.presProps+xml";
	const TABLE_URI: &str =
		"http://schemas.openxmlformats.org/drawingml/2006/table";
	/// The font every run writes in, a code run's in [`MONOSPACE`](Self::MONOSPACE).
	const FONT: &str = "Calibri";
	const MONOSPACE: &str = "Courier New";
	/// The three layouts, each its part, type and name.
	const LAYOUTS: [(Layout, &str, &str); 3] = [
		(Layout::Title, "title", "Title Slide"),
		(Layout::Content, "obj", "Title and Content"),
		(Layout::TwoContent, "twoObj", "Two Content"),
	];

	/// Writes `root` as a slide deck's bytes.
	fn write(
		world: &mut World,
		root: Entity,
		request: &RequestParts,
	) -> Result<Vec<u8>> {
		let mut media = MediaContext::new(Medium::Screen)
			.with_viewport(MediaViewport::new(Self::SLIDE.x, Self::SLIDE.y));
		if let Some(scheme) = OfficeStyleSheet::scheme(world, request) {
			media = media.with_scheme(scheme);
		}
		let styles = OfficeStyleSheet::resolve(world, root, media)?;
		let slides =
			world.with_state::<DeckGather, _>(|gather| gather.slides(root));
		let resolved = Resolved::new(world, styles, &slides)?;
		let mut write = DeckWrite {
			resolved: &resolved,
			package: OpcPackage::default(),
			images: 0,
		};
		write.write_deck(&slides)?;
		let mut package = write.package;
		if let Some(meta) = world.entity(root).get::<DocMeta>() {
			meta.write_package(&mut package);
		}
		package.to_bytes()
	}

	/// The chain a slide's title reads back into under `layout`.
	fn title_chain(layout: Layout) -> Chain {
		let mut chain = vec![ProbeTag::new("article")];
		match layout {
			Layout::Title => {
				chain.push(ProbeTag::with_class("div", "card-center"))
			}
			_ => chain.extend([
				ProbeTag::with_class("div", "card-content"),
				ProbeTag::with_class("header", "card-heading"),
			]),
		}
		chain.push(ProbeTag::new("h1"));
		chain
	}

	/// The chain a slide's body blocks read back into: a title slide's
	/// subtitle in its center, a column in its content, a slide without a
	/// title its article.
	fn body_chain(layout: Option<Layout>) -> Chain {
		let mut chain = vec![ProbeTag::new("article")];
		match layout {
			Some(Layout::Title) => {
				chain.push(ProbeTag::with_class("div", "card-center"))
			}
			Some(_) => chain.extend([
				ProbeTag::with_class("div", "card-content"),
				ProbeTag::with_class("div", "card-columns"),
				ProbeTag::with_class("div", "card-column"),
			]),
			None => {}
		}
		chain
	}
}

impl DeckGather<'_, '_> {
	/// Every slide of `root`: each `article data-card` not inside another,
	/// else the whole document as one.
	fn slides(&self, root: Entity) -> Vec<SlideWrite> {
		let mut cards = Vec::new();
		self.cards(root, &mut cards);
		if cards.is_empty() {
			cards.push(root);
		}
		cards.into_iter().map(|card| self.slide(card)).collect()
	}

	/// The card articles under `entity`, outermost only, in document order.
	fn cards(&self, entity: Entity, out: &mut Vec<Entity>) {
		for child in self.children_of(entity) {
			if self.is_card(child) {
				out.push(child);
			} else {
				self.cards(child, out);
			}
		}
	}

	fn is_card(&self, entity: Entity) -> bool {
		self.elements.get(entity).is_ok_and(|view| {
			view.tag() == "article" && view.attribute("data-card").is_some()
		})
	}

	fn children_of(&self, entity: Entity) -> Vec<Entity> {
		self.tree.children(entity).collect()
	}

	fn tag(&self, entity: Entity) -> Option<&str> {
		self.tags.get(entity).ok().map(|element| element.tag())
	}

	fn has_class(&self, entity: Entity, class: &str) -> bool {
		self.elements
			.get(entity)
			.is_ok_and(|view| view.contains_class(class))
	}

	fn is_heading(&self, entity: Entity) -> bool {
		self.tag(entity).is_some_and(|tag| {
			matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
		})
	}

	fn is_blank(&self, entity: Entity) -> bool {
		self.values
			.get(entity)
			.is_ok_and(|value| value.to_string().trim().is_empty())
	}

	/// The children of `entity` that build anything, whitespace text
	/// between blocks aside.
	fn significant(&self, entity: Entity) -> Vec<Entity> {
		self.children_of(entity)
			.into_iter()
			.filter(|child| !self.is_blank(*child))
			.collect()
	}

	/// One card: its front before its first `hr` child through the layout
	/// its markup names, the rest its notes.
	fn slide(&self, card: Entity) -> SlideWrite {
		let hidden = self.is_card(card)
			&& self
				.elements
				.get(card)
				.is_ok_and(|view| view.attribute("hidden").is_some());
		let children = self.significant(card);
		let split = children
			.iter()
			.position(|child| self.tag(*child) == Some("hr"))
			.unwrap_or(children.len());
		let (front, notes) = children.split_at(split);
		let notes_chain = vec![ProbeTag::new("article")];
		let mut notes_column = ColumnWrite::default();
		for child in notes.iter().skip(1) {
			self.block(*child, &notes_chain, 0, &mut notes_column);
		}
		let mut slide = self.front(front);
		slide.hidden = hidden;
		slide.notes = notes_column.paragraphs;
		slide
	}

	/// A card's front: a lone `card-center` a title slide, a lone
	/// `card-content` a heading over its columns, else a leading heading
	/// over the rest.
	fn front(&self, front: &[Entity]) -> SlideWrite {
		let lone = match front {
			[only] => Some(*only),
			_ => None,
		};
		if let Some(center) =
			lone.filter(|only| self.has_class(*only, "card-center"))
		{
			let blocks = self.significant(center);
			let heading =
				blocks.iter().position(|block| self.is_heading(*block));
			let title = heading.map(|index| {
				self.title(
					blocks[index],
					DeckRenderer::title_chain(Layout::Title),
				)
			});
			let mut column = ColumnWrite::default();
			let chain = DeckRenderer::body_chain(Some(Layout::Title));
			for (index, block) in blocks.iter().enumerate() {
				if Some(index) != heading {
					self.block(*block, &chain, 0, &mut column);
				}
			}
			return SlideWrite {
				hidden: false,
				layout: Layout::Title,
				title,
				subtitle: column.paragraphs,
				columns: match column.objects.is_empty() {
					true => Vec::new(),
					false => vec![ColumnWrite {
						paragraphs: Vec::new(),
						objects: column.objects,
					}],
				},
				notes: Vec::new(),
			};
		}
		if let Some(content) =
			lone.filter(|only| self.has_class(*only, "card-content"))
		{
			let blocks = self.significant(content);
			let header = blocks
				.iter()
				.copied()
				.find(|block| self.has_class(*block, "card-heading"));
			let title_entity = header
				.and_then(|header| {
					self.significant(header)
						.into_iter()
						.find(|child| self.is_heading(*child))
				})
				.or(header);
			let rest = blocks
				.iter()
				.copied()
				.filter(|block| Some(*block) != header)
				.collect::<Vec<_>>();
			let columns = match rest
				.iter()
				.copied()
				.find(|block| self.has_class(*block, "card-columns"))
			{
				Some(columns) => self
					.significant(columns)
					.into_iter()
					.map(|column| self.significant(column))
					.collect::<Vec<_>>(),
				None => vec![rest],
			};
			let layout = match columns.len() >= 2 {
				true => Layout::TwoContent,
				false => Layout::Content,
			};
			return self.titled(title_entity, columns, layout);
		}
		match front
			.first()
			.copied()
			.filter(|first| self.is_heading(*first))
		{
			Some(heading) => self.titled(
				Some(heading),
				vec![front[1..].to_vec()],
				Layout::Content,
			),
			None => {
				let mut column = ColumnWrite::default();
				let chain = DeckRenderer::body_chain(None);
				for block in front {
					self.block(*block, &chain, 0, &mut column);
				}
				SlideWrite {
					hidden: false,
					layout: Layout::Content,
					title: None,
					subtitle: Vec::new(),
					columns: vec![column],
					notes: Vec::new(),
				}
			}
		}
	}

	/// A slide titled by `heading` over `columns` of blocks.
	fn titled(
		&self,
		heading: Option<Entity>,
		columns: Vec<Vec<Entity>>,
		layout: Layout,
	) -> SlideWrite {
		let chain = DeckRenderer::body_chain(Some(layout));
		SlideWrite {
			hidden: false,
			layout,
			title: heading.map(|heading| {
				self.title(heading, DeckRenderer::title_chain(layout))
			}),
			subtitle: Vec::new(),
			columns: columns
				.into_iter()
				.map(|blocks| {
					let mut column = ColumnWrite::default();
					for block in blocks {
						self.block(block, &chain, 0, &mut column);
					}
					column
				})
				.collect(),
			notes: Vec::new(),
		}
	}

	/// The title paragraph of `heading`'s inline content.
	fn title(&self, heading: Entity, chain: Chain) -> ParagraphWrite {
		let mut inline = Vec::new();
		let mut objects = Vec::new();
		for child in self.children_of(heading) {
			self.inline(
				child,
				heading,
				&RunContext::default(),
				&mut inline,
				&mut objects,
			);
		}
		ParagraphWrite {
			kind: ParagraphKind::Plain,
			entity: heading,
			chain,
			inline,
		}
	}

	/// Gathers the block `entity` into `column` at list depth `level`.
	fn block(
		&self,
		entity: Entity,
		chain: &Chain,
		level: u8,
		column: &mut ColumnWrite,
	) {
		let Some(tag) = self.tag(entity) else {
			// loose text at block level is a paragraph of its own
			if !self.is_blank(entity) {
				self.paragraph(
					entity,
					&[entity],
					chain,
					"p",
					ParagraphKind::Plain,
					column,
				);
			}
			return;
		};
		match tag {
			"p" | "figcaption" | "dt" | "dd" | "summary" => {
				let nodes = self.children_of(entity);
				self.paragraph(
					entity,
					&nodes,
					chain,
					"p",
					ParagraphKind::Plain,
					column,
				)
			}
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				let nodes = self.children_of(entity);
				self.paragraph(
					entity,
					&nodes,
					chain,
					"p",
					ParagraphKind::Heading,
					column,
				)
			}
			"ul" | "ol" => {
				let ordered = tag == "ol";
				let mut list_chain = chain.clone();
				list_chain.push(ProbeTag::new(tag));
				for item in self.significant(entity) {
					self.item(item, &list_chain, level, ordered, column);
				}
			}
			"pre" => {
				let text = self.text_of(entity);
				let mut pre_chain = chain.clone();
				pre_chain.push(ProbeTag::new("p"));
				for line in text.trim_end_matches('\n').split('\n') {
					column.paragraphs.push(ParagraphWrite {
						kind: ParagraphKind::Plain,
						entity,
						chain: pre_chain.clone(),
						inline: vec![InlineWrite::Text {
							text: line.to_string(),
							run: RunContext {
								monospace: true,
								..default()
							},
							look: entity,
						}],
					});
				}
			}
			"table" => {
				let table = self.table(entity, chain);
				column.objects.push(ObjectWrite::Table(table));
			}
			"img" => column.objects.push(ObjectWrite::Picture(entity)),
			"hr" => {}
			tag if WordRenderer::BLOCKS.contains(&tag) => {
				self.container(entity, chain, level, column)
			}
			_ => self.paragraph(
				entity,
				&[entity],
				chain,
				"p",
				ParagraphKind::Plain,
				column,
			),
		}
	}

	/// Gathers the children of `entity` as blocks, its runs of inline
	/// content as paragraphs.
	fn container(
		&self,
		entity: Entity,
		chain: &Chain,
		level: u8,
		column: &mut ColumnWrite,
	) {
		let mut pending = Vec::new();
		for child in self.children_of(entity) {
			let is_block = self.tag(child).is_some_and(|tag| {
				WordRenderer::BLOCKS.contains(&tag) || tag == "img"
			});
			if !is_block {
				pending.push(child);
				continue;
			}
			self.flush(entity, &mut pending, chain, column);
			self.block(child, chain, level, column);
		}
		self.flush(entity, &mut pending, chain, column);
	}

	/// The paragraph of the inline nodes `pending` holds.
	fn flush(
		&self,
		parent: Entity,
		pending: &mut Vec<Entity>,
		chain: &Chain,
		column: &mut ColumnWrite,
	) {
		let nodes = core::mem::take(pending);
		if nodes.iter().all(|node| self.is_blank(*node)) {
			return;
		}
		self.paragraph(
			parent,
			&nodes,
			chain,
			"p",
			ParagraphKind::Plain,
			column,
		);
	}

	/// A list item at `level`: its inline content a paragraph, each nested
	/// list a level deeper.
	fn item(
		&self,
		item: Entity,
		chain: &Chain,
		level: u8,
		ordered: bool,
		column: &mut ColumnWrite,
	) {
		let mut item_chain = chain.clone();
		item_chain.push(ProbeTag::new("li"));
		let kind = ParagraphKind::Item { level, ordered };
		let mut inline = Vec::new();
		let mut nested = Vec::new();
		for child in self.children_of(item) {
			match self.tag(child) {
				Some("ul" | "ol") => nested.push(child),
				// a paragraph in an item continues its line
				Some("p") => {
					if !inline.is_empty() {
						inline.push(InlineWrite::Break);
					}
					for node in self.children_of(child) {
						self.inline(
							node,
							child,
							&RunContext::default(),
							&mut inline,
							&mut column.objects,
						);
					}
				}
				_ => self.inline(
					child,
					item,
					&RunContext::default(),
					&mut inline,
					&mut column.objects,
				),
			}
		}
		if !inline.is_empty() {
			column.paragraphs.push(ParagraphWrite {
				kind,
				entity: item,
				chain: item_chain.clone(),
				inline,
			});
		}
		for list in nested {
			self.block(list, &item_chain, level + 1, column);
		}
	}

	/// A paragraph of `nodes`, its alignment `entity`'s, read back as a
	/// `tag` under `chain`.
	fn paragraph(
		&self,
		entity: Entity,
		nodes: &[Entity],
		chain: &Chain,
		tag: &str,
		kind: ParagraphKind,
		column: &mut ColumnWrite,
	) {
		let mut inline = Vec::new();
		for node in nodes {
			self.inline(
				*node,
				entity,
				&RunContext::default(),
				&mut inline,
				&mut column.objects,
			);
		}
		if inline.is_empty() {
			return;
		}
		let mut chain = chain.clone();
		chain.push(ProbeTag::new(tag));
		column.paragraphs.push(ParagraphWrite {
			kind,
			entity,
			chain,
			inline,
		});
	}

	/// Gathers the inline node `entity` under `parent` inside `run`, a
	/// picture among it an object of its own.
	fn inline(
		&self,
		entity: Entity,
		parent: Entity,
		run: &RunContext,
		out: &mut Vec<InlineWrite>,
		objects: &mut Vec<ObjectWrite>,
	) {
		if let Ok(value) = self.values.get(entity) {
			let text = value.to_string();
			// whitespace alone at a paragraph's edge is layout
			if out.is_empty() && text.trim().is_empty() {
				return;
			}
			out.push(InlineWrite::Text {
				text,
				run: run.clone(),
				look: parent,
			});
			return;
		}
		let Some(tag) = self.tag(entity) else {
			return;
		};
		let mut inner = run.clone();
		match tag {
			"br" => {
				out.push(InlineWrite::Break);
				return;
			}
			"img" => {
				objects.push(ObjectWrite::Picture(entity));
				return;
			}
			"a" => {
				inner.link = self.elements.get(entity).ok().and_then(|view| {
					view.attribute("href").map(|href| href.value.to_string())
				});
				inner.tags.push(ProbeTag::new("a"));
			}
			"sup" => {
				inner.baseline = Some(30_000);
				inner.tags.push(ProbeTag::new("sup"));
			}
			"sub" => {
				inner.baseline = Some(-25_000);
				inner.tags.push(ProbeTag::new("sub"));
			}
			"code" | "kbd" | "samp" => inner.monospace = true,
			_ => {}
		}
		inner.ancestors.push(entity);
		for child in self.children_of(entity) {
			self.inline(child, entity, &inner, out, objects);
		}
	}

	/// The text of `entity`'s subtree.
	fn text_of(&self, entity: Entity) -> String {
		let mut out = String::new();
		for node in self.tree.iter_descendants_inclusive_depth_first(entity) {
			if let Ok(value) = self.values.get(node) {
				out.push_str(&value.to_string());
			}
		}
		out
	}

	/// A table's rows by its grid, each merge's covered slots marked.
	fn table(&self, table: Entity, chain: &Chain) -> TableWrite {
		let mut table_chain = chain.clone();
		table_chain.push(ProbeTag::new("table"));
		let (rows, cells) = (self.grid.rows(table), self.grid.cells(table));
		let head = self
			.children_of(table)
			.into_iter()
			.filter(|child| self.tag(*child) == Some("thead"))
			.flat_map(|head| self.children_of(head))
			.collect::<Vec<_>>();
		let header = rows.first().is_some_and(|row| head.contains(row));
		let columns = cells
			.iter()
			.map(|cell| cell.start.x + cell.span.x)
			.max()
			.unwrap_or(1)
			.max(1);
		let mut written = Vec::new();
		for (index, _) in rows.iter().enumerate() {
			let y = index as u32;
			let group = if header && index == 0 {
				"thead"
			} else {
				"tbody"
			};
			let mut row = Vec::new();
			for x in 0..columns {
				let slot = UVec2::new(x, y);
				let Some(cell) = cells.iter().find(|cell| cell.contains(slot))
				else {
					row.push(CellWrite {
						entity: None,
						span: UVec2::ONE,
						covered: (false, false),
						paragraphs: Vec::new(),
					});
					continue;
				};
				if cell.start != slot {
					row.push(CellWrite {
						entity: None,
						span: UVec2::ONE,
						covered: (slot.x > cell.start.x, slot.y > cell.start.y),
						paragraphs: Vec::new(),
					});
					continue;
				}
				let tag = self.tag(cell.entity).unwrap_or("td").to_string();
				let mut cell_chain = table_chain.clone();
				cell_chain.extend([
					ProbeTag::new(group),
					ProbeTag::new("tr"),
					ProbeTag::new(tag),
				]);
				let mut column = ColumnWrite::default();
				self.container(cell.entity, &cell_chain, 0, &mut column);
				// a lone plain paragraph reads back as the cell's content
				if let [paragraph] = column.paragraphs.as_mut_slice()
					&& paragraph.kind == ParagraphKind::Plain
					&& paragraph.entity == cell.entity
				{
					paragraph.chain = cell_chain.clone();
				}
				row.push(CellWrite {
					entity: Some(cell.entity),
					span: cell.span,
					covered: (false, false),
					paragraphs: column.paragraphs,
				});
			}
			written.push(row);
		}
		TableWrite {
			columns,
			header,
			rows: written,
		}
	}
}

impl SlideWrite {
	/// Every paragraph the slide writes.
	fn paragraphs(&self) -> impl Iterator<Item = &ParagraphWrite> {
		self.title
			.iter()
			.chain(&self.subtitle)
			.chain(self.columns.iter().flat_map(|column| {
				column
					.paragraphs
					.iter()
					.chain(column.objects.iter().flat_map(|object| {
						match object {
							ObjectWrite::Table(table) => table
								.rows
								.iter()
								.flatten()
								.flat_map(|cell| &cell.paragraphs)
								.collect::<Vec<_>>(),
							ObjectWrite::Picture(_) => Vec::new(),
						}
					}))
			}))
			.chain(&self.notes)
	}
}

impl Resolved {
	/// Resolves every look the gathered slides write.
	fn new(
		world: &mut World,
		styles: OfficeStyleSheet,
		slides: &[SlideWrite],
	) -> Result<Self> {
		let mut entities = Vec::new();
		let mut chains = vec![
			DeckRenderer::title_chain(Layout::Content),
			DeckRenderer::title_chain(Layout::Title),
			Self::body_text_chain(),
		];
		for slide in slides {
			for paragraph in slide.paragraphs() {
				entities.push(paragraph.entity);
				chains.push(paragraph.chain.clone());
				for inline in &paragraph.inline {
					if let InlineWrite::Text { run, look, .. } = inline {
						entities.push(*look);
						entities.extend(&run.ancestors);
						chains.push(Self::run_chain(&paragraph.chain, run));
					}
				}
			}
			for column in &slide.columns {
				for object in &column.objects {
					match object {
						ObjectWrite::Picture(entity) => entities.push(*entity),
						ObjectWrite::Table(table) => entities.extend(
							table
								.rows
								.iter()
								.flatten()
								.filter_map(|cell| cell.entity),
						),
					}
				}
			}
		}
		entities.sort();
		entities.dedup();
		let mut unique = HashSet::<Chain>::default();
		chains.retain(|chain| unique.insert(chain.clone()));
		let computed = entities
			.iter()
			.copied()
			.zip(styles.computed(world, &entities)?)
			.collect::<HashMap<_, _>>();
		let probes = styles.probe(world, &chains)?;
		let probes = chains.into_iter().zip(probes).collect::<HashMap<_, _>>();
		let images = world.with_state::<PictureQuery, _>(|pictures| {
			pictures.gather(&entities)
		});
		Self {
			styles,
			computed,
			probes,
			images,
		}
		.xok()
	}

	/// The chain a body's list text reads back into, the master's body
	/// style's look.
	fn body_text_chain() -> Chain {
		let mut chain = DeckRenderer::body_chain(Some(Layout::Content));
		chain.extend([ProbeTag::new("ul"), ProbeTag::new("li")]);
		chain
	}

	/// A run's base chain: its paragraph's, then its structural elements.
	fn run_chain(chain: &Chain, run: &RunContext) -> Chain {
		chain.iter().chain(&run.tags).cloned().collect()
	}

	fn computed(&self, entity: Entity) -> ComputedStyle {
		self.computed.get(&entity).cloned().unwrap_or_default()
	}

	fn probe(&self, chain: &[ProbeTag]) -> &[ComputedStyle] {
		self.probes
			.get(chain)
			.map(Vec::as_slice)
			.unwrap_or_default()
	}

	/// The look of `chain`'s last element.
	fn probe_look(&self, chain: &[ProbeTag]) -> OfficeLook {
		self.probe(chain)
			.last()
			.map(OfficeLook::from_computed)
			.unwrap_or_default()
	}

	/// What a run writes beyond what a reader reading it back resolves: its
	/// look against its paragraph's chain and structural elements, a
	/// heading's run bold whatever it resolves.
	fn run_look(
		&self,
		paragraph: &ParagraphWrite,
		run: &RunContext,
		look: Entity,
	) -> OfficeLook {
		let mut own = OfficeLook::from_computed(&self.computed(look));
		own.background =
			run.ancestors.iter().rev().find_map(|ancestor| {
				self.computed(*ancestor).visual.background
			});
		let chain = Self::run_chain(&paragraph.chain, run);
		let styles = self.probe(&chain);
		let mut base = styles
			.last()
			.map(OfficeLook::from_computed)
			.unwrap_or_default();
		base.background = styles
			.iter()
			.skip(paragraph.chain.len())
			.rev()
			.find_map(|style| style.visual.background);
		let mut diff = own.diff(&base);
		diff.text_align = None;
		if paragraph.kind == ParagraphKind::Heading {
			diff.bold = Some(true);
		}
		diff
	}

	/// A paragraph's alignment where it differs from what it reads back.
	fn align(&self, paragraph: &ParagraphWrite) -> Option<&'static str> {
		let own = self.computed(paragraph.entity).visual.text_align;
		let base = self.probe_look(&paragraph.chain).text_align;
		(Some(own) != base)
			.then_some(own)
			.and_then(|align| match align {
				TextAlign::Center => Some("ctr"),
				TextAlign::Right => Some("r"),
				_ => None,
			})
	}

	/// A computed length in px against `of`.
	fn px(&self, length: Length, of: f32) -> f32 {
		OfficeUnits::px(length, of, *self.styles.media().viewport())
	}

	/// The font size a paragraph's text sets in px, for its estimated
	/// height.
	fn font_px(&self, paragraph: &ParagraphWrite) -> f32 {
		self.probe_look(&paragraph.chain)
			.font_size_px
			.unwrap_or(OfficeUnits::PX_PER_REM)
	}
}

impl ParagraphWrite {
	/// The characters of its text, for its estimated height.
	fn chars(&self) -> usize {
		self.inline
			.iter()
			.map(|inline| match inline {
				InlineWrite::Text { text, .. } => text.chars().count(),
				InlineWrite::Break => 1,
			})
			.sum()
	}

	/// Its estimated height in px set `width` wide at `font` px: a glyph
	/// half its size wide, a line 1.2 times its size, each break a line.
	fn height(&self, width: f32, font: f32) -> f32 {
		let per_line = (width / (font * 0.5)).max(1.);
		let breaks = self
			.inline
			.iter()
			.filter(|inline| matches!(inline, InlineWrite::Break))
			.count();
		let lines =
			(self.chars() as f32 / per_line).ceil().max(1.) + breaks as f32;
		lines * font * 1.2 + font * 0.4
	}
}

/// A box on the slide in px, a [`Rect`] from its origin and size.
struct Area;

impl Area {
	fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
		let origin = Vec2::new(x, y);
		Rect::from_corners(origin, origin + Vec2::new(width, height))
	}

	/// `area`'s `xfrm` offset and extent in EMU.
	fn emu(area: &Rect) -> [(String, String); 2] {
		let emu = |px: f32| OfficeUnits::emu(px).to_string();
		let size = area.size();
		[
			(emu(area.min.x), emu(area.min.y)),
			(emu(size.x), emu(size.y)),
		]
	}
}

impl DeckWrite<'_> {
	fn styles(&self) -> &OfficeStyleSheet { &self.resolved.styles }

	/// Writes every part of the deck.
	fn write_deck(&mut self, slides: &[SlideWrite]) -> Result {
		let package = &mut self.package;
		package.insert(
			DeckRenderer::PRESENTATION,
			DeckRenderer::PRESENTATION_TYPE,
			Vec::new(),
		);
		package.relate(
			None,
			OpcRelationship::OFFICE_DOCUMENT,
			DeckRenderer::PRESENTATION,
		);
		let theme = self.resolved.styles.palette().theme_part();
		package.insert(
			DeckRenderer::THEME,
			DeckRenderer::THEME_TYPE,
			theme.clone(),
		);
		package.insert(
			DeckRenderer::NOTES_THEME,
			DeckRenderer::THEME_TYPE,
			theme,
		);
		// the master and its three layouts
		let mut layout_ids = Vec::new();
		for (index, (_, kind, name)) in DeckRenderer::LAYOUTS.iter().enumerate()
		{
			let path = Self::layout_path(index);
			let xml =
				self.layout_xml(DeckRenderer::LAYOUTS[index].0, kind, name);
			self.package
				.insert(path.as_str(), DeckRenderer::LAYOUT_TYPE, xml);
			self.package.relate(
				Some(&path),
				OpcRelationship::SLIDE_MASTER,
				DeckRenderer::MASTER,
			);
			layout_ids.push(self.package.relate(
				Some(DeckRenderer::MASTER),
				OpcRelationship::SLIDE_LAYOUT,
				&path,
			));
		}
		let master = self.master_xml(&layout_ids);
		self.package.insert(
			DeckRenderer::MASTER,
			DeckRenderer::MASTER_TYPE,
			master,
		);
		self.package.relate(
			Some(DeckRenderer::MASTER),
			OpcRelationship::THEME,
			DeckRenderer::THEME,
		);
		let master_id = self.package.relate(
			Some(DeckRenderer::PRESENTATION),
			OpcRelationship::SLIDE_MASTER,
			DeckRenderer::MASTER,
		);
		// the notes master, written whether or not a slide has notes
		let notes_master = Self::notes_master_xml();
		self.package.insert(
			DeckRenderer::NOTES_MASTER,
			DeckRenderer::NOTES_MASTER_TYPE,
			notes_master,
		);
		self.package.relate(
			Some(DeckRenderer::NOTES_MASTER),
			OpcRelationship::THEME,
			DeckRenderer::NOTES_THEME,
		);
		let notes_master_id = self.package.relate(
			Some(DeckRenderer::PRESENTATION),
			OpcRelationship::NOTES_MASTER,
			DeckRenderer::NOTES_MASTER,
		);
		// every slide, its layout and its notes
		let mut slide_ids = Vec::new();
		for (index, slide) in slides.iter().enumerate() {
			let path = format!("ppt/slides/slide{}.xml", index + 1);
			let layout = DeckRenderer::LAYOUTS
				.iter()
				.position(|(layout, ..)| *layout == slide.layout)
				.unwrap_or(1);
			self.package.relate(
				Some(&path),
				OpcRelationship::SLIDE_LAYOUT,
				&Self::layout_path(layout),
			);
			let xml = self.slide_xml(&path, slide);
			self.package
				.insert(path.as_str(), DeckRenderer::SLIDE_TYPE, xml);
			if !slide.notes.is_empty() {
				let notes =
					format!("ppt/notesSlides/notesSlide{}.xml", index + 1);
				let xml = self.notes_xml(&notes, &slide.notes);
				self.package.insert(
					notes.as_str(),
					DeckRenderer::NOTES_SLIDE_TYPE,
					xml,
				);
				self.package.relate(
					Some(&notes),
					OpcRelationship::NOTES_MASTER,
					DeckRenderer::NOTES_MASTER,
				);
				self.package.relate(
					Some(&notes),
					OpcRelationship::SLIDE,
					&path,
				);
				self.package.relate(
					Some(&path),
					OpcRelationship::NOTES_SLIDE,
					&notes,
				);
			}
			slide_ids.push(self.package.relate(
				Some(DeckRenderer::PRESENTATION),
				OpcRelationship::SLIDE,
				&path,
			));
		}
		self.package.insert(
			DeckRenderer::PRES_PROPS,
			DeckRenderer::PRES_PROPS_TYPE,
			format!(
				"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
				 <p:presentationPr xmlns:p=\"{}\"/>",
				OoxmlNamespace::PRESENTATION
			),
		);
		self.package.relate(
			Some(DeckRenderer::PRESENTATION),
			"http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps",
			DeckRenderer::PRES_PROPS,
		);
		self.package.relate(
			Some(DeckRenderer::PRESENTATION),
			OpcRelationship::THEME,
			DeckRenderer::THEME,
		);
		let presentation =
			Self::presentation_xml(&master_id, &notes_master_id, &slide_ids);
		self.package.insert(
			DeckRenderer::PRESENTATION,
			DeckRenderer::PRESENTATION_TYPE,
			presentation,
		);
		Ok(())
	}

	fn layout_path(index: usize) -> String {
		format!("ppt/slideLayouts/slideLayout{}.xml", index + 1)
	}

	/// Opens a part's root `name` with the three namespaces every
	/// PresentationML part binds.
	fn open_part(name: &str) -> XmlPart {
		let mut xml = XmlPart::new();
		xml.open(name, &[
			("xmlns:a", OoxmlNamespace::DRAWING),
			("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
			("xmlns:p", OoxmlNamespace::PRESENTATION),
		]);
		xml
	}

	fn presentation_xml(
		master: &str,
		notes_master: &str,
		slides: &[SmolStr],
	) -> Vec<u8> {
		let mut xml = Self::open_part("p:presentation");
		xml.open("p:sldMasterIdLst", &[])
			.empty("p:sldMasterId", &[("id", "2147483648"), ("r:id", master)])
			.close("p:sldMasterIdLst")
			.open("p:notesMasterIdLst", &[])
			.empty("p:notesMasterId", &[("r:id", notes_master)])
			.close("p:notesMasterIdLst");
		if !slides.is_empty() {
			xml.open("p:sldIdLst", &[]);
			for (index, id) in slides.iter().enumerate() {
				xml.empty("p:sldId", &[
					("id", &(256 + index).to_string()),
					("r:id", id),
				]);
			}
			xml.close("p:sldIdLst");
		}
		xml.empty("p:sldSz", &[("cx", "12192000"), ("cy", "6858000")])
			.empty("p:notesSz", &[("cx", "6858000"), ("cy", "9144000")])
			.close("p:presentation");
		xml.into_bytes()
	}

	/// The areas each placeholder of `layout` sits in: its title, then its
	/// bodies, `columns` of them side by side.
	fn areas(layout: Layout, columns: usize) -> (Rect, Vec<Rect>) {
		let (margin, gap, slide) =
			(DeckRenderer::MARGIN, DeckRenderer::GAP, DeckRenderer::SLIDE);
		let width = slide.x - margin * 2.;
		match layout {
			Layout::Title => (
				Area::new(
					margin * 2.,
					slide.y * 0.28,
					slide.x - margin * 4.,
					slide.y * 0.24,
				),
				vec![Area::new(
					margin * 2.,
					slide.y * 0.54,
					slide.x - margin * 4.,
					slide.y * 0.3,
				)],
			),
			_ => {
				let top = margin + DeckRenderer::TITLE_HEIGHT + gap;
				let height = slide.y - top - margin;
				let columns = columns.max(match layout {
					Layout::TwoContent => 2,
					_ => 1,
				});
				let column =
					(width - gap * (columns - 1) as f32) / columns as f32;
				(
					Area::new(
						margin,
						margin,
						width,
						DeckRenderer::TITLE_HEIGHT,
					),
					(0..columns)
						.map(|index| {
							Area::new(
								margin + index as f32 * (column + gap),
								top,
								column,
								height,
							)
						})
						.collect(),
				)
			}
		}
	}

	/// Writes a shape's non-visual properties and frame, a placeholder of
	/// `kind` and `index` where given.
	fn shape_header(
		xml: &mut XmlPart,
		id: u32,
		name: &str,
		placeholder: Option<(Option<&str>, Option<u32>)>,
		area: &Rect,
	) {
		let id = id.to_string();
		xml.open("p:nvSpPr", &[])
			.empty("p:cNvPr", &[("id", &id), ("name", name)])
			.open("p:cNvSpPr", &[]);
		if placeholder.is_some() {
			xml.empty("a:spLocks", &[("noGrp", "1")]);
		}
		xml.close("p:cNvSpPr").open("p:nvPr", &[]);
		if let Some((kind, index)) = placeholder {
			let index = index.map(|index| index.to_string());
			let mut attributes = Vec::new();
			if let Some(kind) = kind {
				attributes.push(("type", kind));
			}
			if let Some(index) = &index {
				attributes.push(("idx", index.as_str()));
			}
			xml.empty("p:ph", &attributes);
		}
		let [(x, y), (cx, cy)] = Area::emu(area);
		xml.close("p:nvPr")
			.close("p:nvSpPr")
			.open("p:spPr", &[])
			.open("a:xfrm", &[])
			.empty("a:off", &[("x", &x), ("y", &y)])
			.empty("a:ext", &[("cx", &cx), ("cy", &cy)])
			.close("a:xfrm")
			.close("p:spPr");
	}

	/// Opens a shape tree with its own group properties.
	fn open_tree(xml: &mut XmlPart) {
		xml.open("p:spTree", &[])
			.open("p:nvGrpSpPr", &[])
			.empty("p:cNvPr", &[("id", "1"), ("name", "")])
			.empty("p:cNvGrpSpPr", &[])
			.empty("p:nvPr", &[])
			.close("p:nvGrpSpPr")
			.open("p:grpSpPr", &[])
			.open("a:xfrm", &[])
			.empty("a:off", &[("x", "0"), ("y", "0")])
			.empty("a:ext", &[("cx", "0"), ("cy", "0")])
			.empty("a:chOff", &[("x", "0"), ("y", "0")])
			.empty("a:chExt", &[("cx", "0"), ("cy", "0")])
			.close("a:xfrm")
			.close("p:grpSpPr");
	}

	/// A placeholder shape with no text of its own, for a layout or master.
	fn empty_placeholder(
		xml: &mut XmlPart,
		id: u32,
		name: &str,
		kind: Option<&str>,
		index: Option<u32>,
		area: &Rect,
		list_style: &str,
	) {
		xml.open("p:sp", &[]);
		Self::shape_header(xml, id, name, Some((kind, index)), area);
		xml.open("p:txBody", &[])
			.empty("a:bodyPr", &[])
			.raw(list_style)
			.open("a:p", &[])
			.empty("a:endParaRPr", &[("lang", "en-US")])
			.close("a:p")
			.close("p:txBody")
			.close("p:sp");
	}

	/// The master: the root's background, the title style from the card
	/// heading's look and the body style from the body's.
	fn master_xml(&self, layouts: &[SmolStr]) -> Vec<u8> {
		let mut xml = Self::open_part("p:sldMaster");
		xml.open("p:cSld", &[]);
		match self.styles().defaults().background {
			Some(background) => {
				xml.open("p:bg", &[]).open("p:bgPr", &[]);
				self.fill(&mut xml, background);
				xml.empty("a:effectLst", &[]).close("p:bgPr").close("p:bg");
			}
			None => {
				xml.open("p:bg", &[])
					.open("p:bgRef", &[("idx", "1001")])
					.empty("a:schemeClr", &[("val", "bg1")])
					.close("p:bgRef")
					.close("p:bg");
			}
		}
		Self::open_tree(&mut xml);
		let (title, bodies) = Self::areas(Layout::Content, 1);
		Self::empty_placeholder(
			&mut xml,
			2,
			"Title Placeholder 1",
			Some("title"),
			None,
			&title,
			"<a:lstStyle/>",
		);
		Self::empty_placeholder(
			&mut xml,
			3,
			"Text Placeholder 2",
			Some("body"),
			Some(1),
			&bodies[0],
			"<a:lstStyle/>",
		);
		xml.close("p:spTree").close("p:cSld").empty("p:clrMap", &[
			("bg1", "lt1"),
			("tx1", "dk1"),
			("bg2", "lt2"),
			("tx2", "dk2"),
			("accent1", "accent1"),
			("accent2", "accent2"),
			("accent3", "accent3"),
			("accent4", "accent4"),
			("accent5", "accent5"),
			("accent6", "accent6"),
			("hlink", "hlink"),
			("folHlink", "folHlink"),
		]);
		xml.open("p:sldLayoutIdLst", &[]);
		for (index, id) in layouts.iter().enumerate() {
			xml.empty("p:sldLayoutId", &[
				("id", &(2147483649u32 + index as u32).to_string()),
				("r:id", id),
			]);
		}
		xml.close("p:sldLayoutIdLst").open("p:txStyles", &[]);
		let title = self
			.resolved
			.probe_look(&DeckRenderer::title_chain(Layout::Content));
		let body = self.resolved.probe_look(&Resolved::body_text_chain());
		xml.open("p:titleStyle", &[]).open("a:lvl1pPr", &[]);
		self.default_run(&mut xml, &title, true);
		xml.close("a:lvl1pPr")
			.close("p:titleStyle")
			.open("p:bodyStyle", &[]);
		for level in 0..9u32 {
			let indent = OfficeUnits::emu(24. * (level + 1) as f32).to_string();
			let name = format!("a:lvl{}pPr", level + 1);
			xml.open(&name, &[("marL", &indent), ("indent", "-228600")])
				.empty("a:buFont", &[("typeface", "Arial")])
				.empty("a:buChar", &[("char", "•")]);
			self.default_run(&mut xml, &body, false);
			xml.close(&name);
		}
		xml.close("p:bodyStyle")
			.open("p:otherStyle", &[])
			.open("a:lvl1pPr", &[]);
		self.default_run(&mut xml, &body, false);
		xml.close("a:lvl1pPr")
			.close("p:otherStyle")
			.close("p:txStyles")
			.close("p:sldMaster");
		xml.into_bytes()
	}

	/// A text style level's default run: `look`'s size, weight and colour.
	fn default_run(&self, xml: &mut XmlPart, look: &OfficeLook, title: bool) {
		let size = look.font_size_px.map(|px| {
			((OfficeUnits::pt(px) * 100.).round() as u32).to_string()
		});
		let mut attributes = vec![("lang", "en-US".to_string())];
		if let Some(size) = size {
			attributes.push(("sz", size));
		}
		attributes.push((
			"b",
			match look.bold.unwrap_or(false) {
				true => "1",
				false => "0",
			}
			.into(),
		));
		let attributes = attributes
			.iter()
			.map(|(key, value)| (*key, value.as_str()))
			.collect::<Vec<_>>();
		xml.open("a:defRPr", &attributes);
		match look.color {
			Some(color) => self.fill(xml, color),
			None => {
				xml.open("a:solidFill", &[])
					.empty("a:schemeClr", &[("val", "tx1")])
					.close("a:solidFill");
			}
		}
		let font = match title {
			true => "+mj-lt",
			false => "+mn-lt",
		};
		xml.empty("a:latin", &[("typeface", font)])
			.close("a:defRPr");
	}

	/// A layout: its placeholders' frames, its name and type.
	fn layout_xml(&self, layout: Layout, kind: &str, name: &str) -> Vec<u8> {
		let mut xml = XmlPart::new();
		xml.open("p:sldLayout", &[
			("xmlns:a", OoxmlNamespace::DRAWING),
			("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
			("xmlns:p", OoxmlNamespace::PRESENTATION),
			("type", kind),
			("preserve", "1"),
		])
		.open("p:cSld", &[("name", name)]);
		Self::open_tree(&mut xml);
		let (title, bodies) = Self::areas(layout, 1);
		match layout {
			Layout::Title => {
				Self::empty_placeholder(
					&mut xml,
					2,
					"Title 1",
					Some("ctrTitle"),
					None,
					&title,
					"<a:lstStyle><a:lvl1pPr algn=\"ctr\"/></a:lstStyle>",
				);
				Self::empty_placeholder(
					&mut xml,
					3,
					"Subtitle 2",
					Some("subTitle"),
					Some(1),
					&bodies[0],
					"<a:lstStyle><a:lvl1pPr marL=\"0\" indent=\"0\" algn=\"ctr\"><a:buNone/></a:lvl1pPr></a:lstStyle>",
				);
			}
			_ => {
				Self::empty_placeholder(
					&mut xml,
					2,
					"Title 1",
					Some("title"),
					None,
					&title,
					"<a:lstStyle/>",
				);
				for (index, body) in bodies.iter().enumerate() {
					Self::empty_placeholder(
						&mut xml,
						3 + index as u32,
						&format!("Content Placeholder {}", index + 2),
						None,
						Some(index as u32 + 1),
						body,
						"<a:lstStyle/>",
					);
				}
			}
		}
		xml.close("p:spTree")
			.close("p:cSld")
			.open("p:clrMapOvr", &[])
			.empty("a:masterClrMapping", &[])
			.close("p:clrMapOvr")
			.close("p:sldLayout");
		xml.into_bytes()
	}

	/// A slide: its title, subtitle or columns, each column's text frame
	/// shrunk to its estimated height so its pictures and tables fit below.
	fn slide_xml(&mut self, part: &str, slide: &SlideWrite) -> Vec<u8> {
		let mut attributes = vec![
			("xmlns:a", OoxmlNamespace::DRAWING),
			("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
			("xmlns:p", OoxmlNamespace::PRESENTATION),
		];
		if slide.hidden {
			attributes.push(("show", "0"));
		}
		let mut xml = XmlPart::new();
		xml.open("p:sld", &attributes).open("p:cSld", &[]);
		Self::open_tree(&mut xml);
		let mut id = 2;
		let columns = slide.columns.len().max(1);
		let (title_area, body_areas) = Self::areas(slide.layout, columns);
		if let Some(title) = &slide.title {
			let kind = match slide.layout {
				Layout::Title => "ctrTitle",
				_ => "title",
			};
			self.text_shape(
				part,
				&mut xml,
				id,
				"Title",
				Some((Some(kind), None)),
				&title_area,
				std::slice::from_ref(title),
				false,
			);
			id += 1;
		}
		if !slide.subtitle.is_empty() {
			self.text_shape(
				part,
				&mut xml,
				id,
				"Subtitle",
				Some((Some("subTitle"), Some(1))),
				&body_areas[0],
				&slide.subtitle,
				false,
			);
			id += 1;
		}
		// a title slide's pictures and tables sit below its subtitle
		let column_areas = match slide.layout {
			Layout::Title => {
				let below = body_areas[0].max.y;
				vec![Area::new(
					DeckRenderer::MARGIN,
					below,
					DeckRenderer::SLIDE.x - DeckRenderer::MARGIN * 2.,
					(DeckRenderer::SLIDE.y - below - DeckRenderer::MARGIN)
						.max(0.),
				)]
			}
			_ => body_areas,
		};
		for (index, (column, area)) in
			slide.columns.iter().zip(&column_areas).enumerate()
		{
			let mut top = area.min.y;
			if !column.paragraphs.is_empty() {
				let text_height = match column.objects.is_empty() {
					true => area.height(),
					false => {
						column
							.paragraphs
							.iter()
							.map(|paragraph| {
								paragraph.height(
									area.width(),
									self.resolved.font_px(paragraph),
								)
							})
							.sum::<f32>()
							.min(area.height()) + DeckRenderer::GAP
					}
				};
				let text_area =
					Area::new(area.min.x, top, area.width(), text_height);
				let placeholder = match slide.layout {
					Layout::Title => None,
					_ => Some((None, Some(index as u32 + 1))),
				};
				self.text_shape(
					part,
					&mut xml,
					id,
					"Content",
					placeholder,
					&text_area,
					&column.paragraphs,
					placeholder.is_some(),
				);
				id += 1;
				top += text_height + DeckRenderer::GAP;
			}
			// each picture and table its share of what remains
			let remaining = (area.max.y - top).max(0.);
			let count = column.objects.len().max(1) as f32;
			let share = ((remaining - DeckRenderer::GAP * (count - 1.))
				/ count)
				.max(0.);
			for object in &column.objects {
				let bounds = Area::new(area.min.x, top, area.width(), share);
				let height = match object {
					ObjectWrite::Picture(entity) => {
						self.picture(part, &mut xml, id, *entity, &bounds)
					}
					ObjectWrite::Table(table) => {
						self.table(part, &mut xml, id, table, &bounds)
					}
				};
				id += 1;
				top += height + DeckRenderer::GAP;
			}
		}
		xml.close("p:spTree")
			.close("p:cSld")
			.open("p:clrMapOvr", &[])
			.empty("a:masterClrMapping", &[])
			.close("p:clrMapOvr")
			.close("p:sld");
		xml.into_bytes()
	}

	/// A text shape of `paragraphs`, a body placeholder's plain paragraphs
	/// unbulleted.
	fn text_shape(
		&mut self,
		part: &str,
		xml: &mut XmlPart,
		id: u32,
		name: &str,
		placeholder: Option<(Option<&str>, Option<u32>)>,
		area: &Rect,
		paragraphs: &[ParagraphWrite],
		body: bool,
	) {
		xml.open("p:sp", &[]);
		Self::shape_header(xml, id, &format!("{name} {id}"), placeholder, area);
		xml.open("p:txBody", &[])
			.open("a:bodyPr", &[("wrap", "square")])
			.empty("a:normAutofit", &[])
			.close("a:bodyPr")
			.empty("a:lstStyle", &[]);
		for paragraph in paragraphs {
			self.paragraph(part, xml, paragraph, body);
		}
		xml.close("p:txBody").close("p:sp");
	}

	/// One paragraph: its level, bullet and alignment, then its runs.
	fn paragraph(
		&mut self,
		part: &str,
		xml: &mut XmlPart,
		paragraph: &ParagraphWrite,
		body: bool,
	) {
		xml.open("a:p", &[]);
		let align = self.resolved.align(paragraph);
		let mut attributes = Vec::<(&str, String)>::new();
		let bullet = match paragraph.kind {
			ParagraphKind::Item { level, ordered } => {
				if level > 0 {
					attributes.push(("lvl", level.to_string()));
				}
				// a body bullets by its master, any other frame says so
				match (ordered, body) {
					(true, _) => Some("<a:buAutoNum type=\"arabicPeriod\"/>"),
					(false, true) => None,
					(false, false) => Some(
						"<a:buFont typeface=\"Arial\"/><a:buChar char=\"•\"/>",
					),
				}
			}
			_ if body => {
				attributes
					.extend([("marL", "0".into()), ("indent", "0".into())]);
				Some("<a:buNone/>")
			}
			_ => None,
		};
		if let ParagraphKind::Item { level, .. } = paragraph.kind
			&& !body
		{
			let indent = OfficeUnits::emu(24. * (level + 1) as f32).to_string();
			attributes.extend([("marL", indent), ("indent", "-228600".into())]);
		}
		if let Some(align) = align {
			attributes.push(("algn", align.into()));
		}
		if !attributes.is_empty() || bullet.is_some() {
			let attributes = attributes
				.iter()
				.map(|(key, value)| (*key, value.as_str()))
				.collect::<Vec<_>>();
			xml.open("a:pPr", &attributes);
			if let Some(bullet) = bullet {
				xml.raw(bullet);
			}
			xml.close("a:pPr");
		}
		for inline in &paragraph.inline {
			match inline {
				InlineWrite::Text { text, run, look } => {
					let properties =
						self.resolved.run_look(paragraph, run, *look);
					xml.open("a:r", &[]);
					self.run_properties(part, xml, &properties, run);
					xml.text_element("a:t", &[], text).close("a:r");
				}
				InlineWrite::Break => {
					xml.empty("a:br", &[]);
				}
			}
		}
		xml.empty("a:endParaRPr", &[("lang", "en-US")]).close("a:p");
	}

	/// A run's properties, in the order the schema requires.
	fn run_properties(
		&mut self,
		part: &str,
		xml: &mut XmlPart,
		look: &OfficeLook,
		run: &RunContext,
	) {
		let flag =
			|value: Option<bool>| value.map(|on| if on { "1" } else { "0" });
		let size = look.font_size_px.map(|px| {
			((OfficeUnits::pt(px) * 100.).round() as u32).to_string()
		});
		let baseline = run.baseline.map(|baseline| baseline.to_string());
		let mut attributes = vec![("lang", "en-US")];
		if let Some(size) = &size {
			attributes.push(("sz", size));
		}
		if let Some(bold) = flag(look.bold) {
			attributes.push(("b", bold));
		}
		if let Some(italic) = flag(look.italic) {
			attributes.push(("i", italic));
		}
		match look.underline {
			Some(true) => attributes.push(("u", "sng")),
			Some(false) => attributes.push(("u", "none")),
			None => {}
		}
		match look.strike {
			Some(true) => attributes.push(("strike", "sngStrike")),
			Some(false) => attributes.push(("strike", "noStrike")),
			None => {}
		}
		if let Some(baseline) = &baseline {
			attributes.push(("baseline", baseline));
		}
		let link = run.link.as_ref().map(|href| {
			self.package.relate_external(
				Some(part),
				OpcRelationship::HYPERLINK,
				href,
			)
		});
		xml.open("a:rPr", &attributes);
		if let Some(color) = look.color {
			self.fill(xml, color);
		}
		if let Some(background) = look.background {
			xml.open("a:highlight", &[]);
			self.color(xml, background);
			xml.close("a:highlight");
		}
		let font = match run.monospace {
			true => DeckRenderer::MONOSPACE,
			false => DeckRenderer::FONT,
		};
		if run.monospace {
			xml.empty("a:latin", &[("typeface", font)])
				.empty("a:cs", &[("typeface", font)]);
		}
		if let Some(link) = link {
			xml.empty("a:hlinkClick", &[("r:id", link.as_str())]);
		}
		xml.close("a:rPr");
	}

	/// A solid fill of `color`.
	fn fill(&self, xml: &mut XmlPart, color: Color) {
		xml.open("a:solidFill", &[]);
		self.color(xml, color);
		xml.close("a:solidFill");
	}

	/// `color` as its theme slot where one holds it, else its value.
	fn color(&self, xml: &mut XmlPart, color: Color) {
		match self.styles().theme_slot(color) {
			Some(slot) => xml.empty("a:schemeClr", &[("val", slot.name())]),
			None => {
				xml.empty("a:srgbClr", &[("val", &OfficeColor::hex(color))])
			}
		};
	}

	/// A picture shape fitted to `bounds`, answering its height in px.
	fn picture(
		&mut self,
		part: &str,
		xml: &mut XmlPart,
		id: u32,
		entity: Entity,
		bounds: &Rect,
	) -> f32 {
		let gathered = self
			.resolved
			.images
			.get(&entity)
			.cloned()
			.unwrap_or_else(|| {
				OfficePicture::new(PictureSource::of(""), "", Vec2::ZERO)
			});
		let style = self.resolved.computed(entity);
		let resolved = Vec2::new(
			style
				.box_style
				.width
				.map(|width| self.resolved.px(width, bounds.width()))
				.unwrap_or(0.),
			style
				.box_style
				.height
				.map(|height| self.resolved.px(height, bounds.height()))
				.unwrap_or(0.),
		);
		let picture = gathered.sized(resolved, bounds.size());
		let blip = match &picture.source {
			PictureSource::Embedded(bytes) => {
				self.images += 1;
				let path = format!(
					"ppt/media/image{}.{}",
					self.images,
					OfficePicture::extension(bytes.media_type())
				);
				self.package.insert(
					path.as_str(),
					bytes.media_type().as_str(),
					bytes.bytes().to_vec(),
				);
				(
					"r:embed",
					self.package.relate(
						Some(part),
						OpcRelationship::IMAGE,
						&path,
					),
				)
			}
			PictureSource::Linked(src) => (
				"r:link",
				self.package.relate_external(
					Some(part),
					OpcRelationship::IMAGE,
					src,
				),
			),
		};
		let area = Rect::from_corners(bounds.min, bounds.min + picture.size);
		let [(x, y), (cx, cy)] = Area::emu(&area);
		let id = id.to_string();
		let name = format!("Picture {id}");
		xml.open("p:pic", &[])
			.open("p:nvPicPr", &[])
			.empty("p:cNvPr", &[
				("id", &id),
				("name", &name),
				("descr", &picture.alt),
			])
			.open("p:cNvPicPr", &[])
			.empty("a:picLocks", &[("noChangeAspect", "1")])
			.close("p:cNvPicPr")
			.empty("p:nvPr", &[])
			.close("p:nvPicPr")
			.open("p:blipFill", &[])
			.empty("a:blip", &[(blip.0, blip.1.as_str())])
			.open("a:stretch", &[])
			.empty("a:fillRect", &[])
			.close("a:stretch")
			.close("p:blipFill")
			.open("p:spPr", &[])
			.open("a:xfrm", &[])
			.empty("a:off", &[("x", &x), ("y", &y)])
			.empty("a:ext", &[("cx", &cx), ("cy", &cy)])
			.close("a:xfrm")
			.open("a:prstGeom", &[("prst", "rect")])
			.empty("a:avLst", &[])
			.close("a:prstGeom")
			.close("p:spPr")
			.close("p:pic");
		picture.size.y
	}

	/// A table's graphic frame as wide as `bounds`, a row's height its
	/// tallest cell's estimate, answering its height in px.
	fn table(
		&mut self,
		part: &str,
		xml: &mut XmlPart,
		id: u32,
		table: &TableWrite,
		bounds: &Rect,
	) -> f32 {
		let column = bounds.width() / table.columns as f32;
		let heights = table
			.rows
			.iter()
			.map(|row| {
				row.iter()
					.flat_map(|cell| &cell.paragraphs)
					.map(|paragraph| {
						paragraph
							.height(column, self.resolved.font_px(paragraph))
					})
					.fold(0f32, f32::max)
					.max(24.) + 16.
			})
			.collect::<Vec<_>>();
		let height = heights.iter().sum::<f32>();
		let area =
			Area::new(bounds.min.x, bounds.min.y, bounds.width(), height);
		let [(x, y), (cx, cy)] = Area::emu(&area);
		let id = id.to_string();
		let name = format!("Table {id}");
		xml.open("p:graphicFrame", &[])
			.open("p:nvGraphicFramePr", &[])
			.empty("p:cNvPr", &[("id", &id), ("name", &name)])
			.open("p:cNvGraphicFramePr", &[])
			.empty("a:graphicFrameLocks", &[("noGrp", "1")])
			.close("p:cNvGraphicFramePr")
			.empty("p:nvPr", &[])
			.close("p:nvGraphicFramePr")
			.open("p:xfrm", &[])
			.empty("a:off", &[("x", &x), ("y", &y)])
			.empty("a:ext", &[("cx", &cx), ("cy", &cy)])
			.close("p:xfrm")
			.open("a:graphic", &[])
			.open("a:graphicData", &[("uri", DeckRenderer::TABLE_URI)])
			.open("a:tbl", &[])
			.empty("a:tblPr", &[("firstRow", match table.header {
				true => "1",
				false => "0",
			})])
			.open("a:tblGrid", &[]);
		let width = OfficeUnits::emu(column).to_string();
		for _ in 0..table.columns {
			xml.empty("a:gridCol", &[("w", &width)]);
		}
		xml.close("a:tblGrid");
		let border = self.styles().defaults().color;
		for (row, height) in table.rows.iter().zip(&heights) {
			xml.open("a:tr", &[("h", &OfficeUnits::emu(*height).to_string())]);
			for cell in row {
				let (span_x, span_y) =
					(cell.span.x.to_string(), cell.span.y.to_string());
				let mut attributes = Vec::new();
				if cell.span.x > 1 {
					attributes.push(("gridSpan", span_x.as_str()));
				}
				if cell.span.y > 1 {
					attributes.push(("rowSpan", span_y.as_str()));
				}
				if cell.covered.0 {
					attributes.push(("hMerge", "1"));
				}
				if cell.covered.1 {
					attributes.push(("vMerge", "1"));
				}
				xml.open("a:tc", &attributes)
					.open("a:txBody", &[])
					.empty("a:bodyPr", &[])
					.empty("a:lstStyle", &[]);
				for paragraph in &cell.paragraphs {
					self.paragraph(part, xml, paragraph, false);
				}
				if cell.paragraphs.is_empty() {
					xml.open("a:p", &[])
						.empty("a:endParaRPr", &[("lang", "en-US")])
						.close("a:p");
				}
				xml.close("a:txBody").open("a:tcPr", &[]);
				for side in ["a:lnL", "a:lnR", "a:lnT", "a:lnB"] {
					xml.open(side, &[("w", "9525")]);
					match border {
						Some(color) => self.fill(xml, color),
						None => {
							xml.open("a:solidFill", &[])
								.empty("a:schemeClr", &[("val", "tx1")])
								.close("a:solidFill");
						}
					}
					xml.close(side);
				}
				if let Some(background) = cell.entity.and_then(|entity| {
					self.resolved.computed(entity).visual.background
				}) {
					self.fill(xml, background);
				}
				xml.close("a:tcPr").close("a:tc");
			}
			xml.close("a:tr");
		}
		xml.close("a:tbl")
			.close("a:graphicData")
			.close("a:graphic")
			.close("p:graphicFrame");
		height
	}

	/// A notes slide: the slide's image and its notes in the body
	/// placeholder.
	fn notes_xml(&mut self, part: &str, notes: &[ParagraphWrite]) -> Vec<u8> {
		let mut xml = Self::open_part("p:notes");
		xml.open("p:cSld", &[]);
		Self::open_tree(&mut xml);
		let image = Area::new(48., 72., 624., 351.);
		let body = Area::new(72., 456., 576., 432.);
		xml.open("p:sp", &[]);
		Self::shape_header(
			&mut xml,
			2,
			"Slide Image Placeholder 1",
			Some((Some("sldImg"), None)),
			&image,
		);
		xml.close("p:sp");
		self.text_shape(
			part,
			&mut xml,
			3,
			"Notes Placeholder",
			Some((Some("body"), Some(1))),
			&body,
			notes,
			false,
		);
		xml.close("p:spTree")
			.close("p:cSld")
			.open("p:clrMapOvr", &[])
			.empty("a:masterClrMapping", &[])
			.close("p:clrMapOvr")
			.close("p:notes");
		xml.into_bytes()
	}

	/// The notes master: a slide image and a notes body.
	fn notes_master_xml() -> Vec<u8> {
		let mut xml = Self::open_part("p:notesMaster");
		xml.open("p:cSld", &[]);
		Self::open_tree(&mut xml);
		let image = Area::new(48., 72., 624., 351.);
		let body = Area::new(72., 456., 576., 432.);
		xml.open("p:sp", &[]);
		Self::shape_header(
			&mut xml,
			2,
			"Slide Image Placeholder 1",
			Some((Some("sldImg"), None)),
			&image,
		);
		xml.close("p:sp");
		Self::empty_placeholder(
			&mut xml,
			3,
			"Notes Placeholder 2",
			Some("body"),
			Some(1),
			&body,
			"<a:lstStyle/>",
		);
		xml.close("p:spTree").close("p:cSld").empty("p:clrMap", &[
			("bg1", "lt1"),
			("tx1", "dk1"),
			("bg2", "lt2"),
			("tx2", "dk2"),
			("accent1", "accent1"),
			("accent2", "accent2"),
			("accent3", "accent3"),
			("accent4", "accent4"),
			("accent5", "accent5"),
			("accent6", "accent6"),
			("hlink", "hlink"),
			("folHlink", "folHlink"),
		]);
		xml.close("p:notesMaster");
		xml.into_bytes()
	}
}

impl Installer for DeckRenderer {
	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
}

impl DocRenderer for DeckRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Pptx] }

	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Pptx])?;
		let (root, request) = (cx.entity, cx.request);
		let bytes = Self::write(cx.world, root, request)?;
		MediaBytes::new(MediaType::Pptx, bytes).xok()
	}
}

#[cfg(test)]
mod test {
	use super::super::deck_parser::test::*;
	use super::super::office_test::*;
	use crate::prelude::*;
	use crate::style::material::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// `root` written as a deck for `request`.
	fn write(
		world: &mut World,
		root: Entity,
		request: RequestParts,
	) -> MediaBytes {
		super::super::office_test::write(world, root, MediaType::Pptx, request)
			.unwrap()
	}

	/// A world of the formats, themed by Material when `themed`.
	fn formats(themed: bool) -> World {
		match themed {
			true => (
				TemplatePlugin,
				DocumentPlugin,
				OoxmlPlugin,
				MaterialStylePlugin,
			)
				.into_world(),
			false => (TemplatePlugin, DocumentPlugin, OoxmlPlugin).into_world(),
		}
	}

	/// `bytes` read, written and read again in a fresh world each time,
	/// answering each read's HTML and the written deck.
	fn round_trip(
		bytes: &MediaBytes,
		themed: bool,
		options: OfficeReadOptions,
	) -> (String, String, MediaBytes) {
		let read = |bytes: &MediaBytes| {
			let mut world = formats(themed);
			world
				.resource_mut::<DocParsers>()
				.register(DeckParser::default().with_options(options));
			parse_in(world, bytes).unwrap()
		};
		let (mut first, root) = read(bytes);
		let written = write(&mut first, root, default());
		let (mut again, again_root) = read(&written);
		again
			.entity(again_root)
			.get::<DocMeta>()
			.cloned()
			.xpect_eq(first.entity(root).get::<DocMeta>().cloned());
		(
			html(&mut first, root),
			html(&mut again, again_root),
			written,
		)
	}

	/// The model's round trip: the covering deck read, written and read
	/// again is the same tree, with the Material theme and without, where a
	/// theme slot reads verbatim since no token would resolve.
	#[beet_core::test]
	fn round_trips_the_model() {
		let bytes = covering();
		let (first, again, written) = round_trip(&bytes, true, default());
		again.xpect_eq(first);
		let (first, again, _) = round_trip(
			&bytes,
			false,
			OfficeReadOptions::default()
				.with_theme_colors(ThemeColors::Verbatim),
		);
		again.xpect_eq(first);
		part(&written, "ppt/presentation.xml")
			.xpect_contains("<p:sldSz cx=\"12192000\" cy=\"6858000\"/>")
			.xpect_contains("<p:notesMasterIdLst>");
		part(&written, "ppt/slides/slide1.xml")
			.xpect_contains("<p:ph type=\"ctrTitle\"/>");
		part(&written, "ppt/slides/slide2.xml")
			.xpect_contains("<p:ph idx=\"1\"/>")
			.xpect_contains("<p:ph idx=\"2\"/>");
		part(&written, "ppt/slides/slide5.xml")
			.xpect_contains("<p:sld ")
			.xpect_contains(" show=\"0\"");
		part(&written, "ppt/slideLayouts/slideLayout3.xml")
			.xpect_contains("<p:cSld name=\"Two Content\">");
		part(&written, "ppt/notesSlides/notesSlide2.xml")
			.xpect_contains("First point.");
	}

	/// An authored stack of card articles writes a slide per card, each
	/// card's notes after its `hr` its speaker notes, and reads back so.
	#[beet_core::test]
	fn writes_authored_cards() {
		let html_source = "<article data-card><h1>Intro</h1><p>Hello <strong>there</strong></p><ul><li>one</li><li>two</li></ul><hr/><p>Say hi.</p></article>\
			<article data-card><p>No heading here</p><hr/><p>Pause.</p></article>";
		let (mut world, root) = parse_in(
			formats(false),
			&MediaBytes::new(MediaType::Html, html_source.as_bytes().to_vec()),
		)
		.unwrap();
		let written = write(&mut world, root, default());
		let (mut again, again_root) =
			parse_in(formats(false), &written).unwrap();
		let read = html(&mut again, again_root);
		read.matches("<article data-card>").count().xpect_eq(2);
		read.clone()
			// a leading heading titles its slide
			.xpect_contains("<header class=\"card-heading\"><h1>Intro</h1></header><div class=\"card-columns\"><div class=\"card-column\"><p>Hello <strong>there</strong></p><ul><li>one</li><li>two</li></ul></div></div></div><hr /><p>Say hi.</p>")
			.xpect_contains("<article data-card><p>No heading here</p><hr /><p>Pause.</p></article>");
		// and that read round trips
		let rewritten = write(&mut again, again_root, default());
		let (mut third, third_root) =
			parse_in(formats(false), &rewritten).unwrap();
		html(&mut third, third_root).xpect_eq(read);
	}

	/// Cards a layout transcludes through a [`Portal`] write as the render
	/// tree has them, a slide each.
	#[beet_core::test]
	fn writes_cards_through_a_portal() {
		let mut world = formats(false);
		let content = world
			.spawn(rsx! {
				<article data-card><h1>"One"</h1></article>
				<article data-card><h1>"Two"</h1></article>
			})
			.id();
		let root = world
			.spawn((Element::new("main"), children![Portal::new(content)]))
			.id();
		let written = write(&mut world, root, default());
		let (mut again, again_root) =
			parse_in(formats(false), &written).unwrap();
		let read = html(&mut again, again_root);
		read.matches("<article data-card>").count().xpect_eq(2);
		read.xpect_contains("One").xpect_contains("Two");
	}

	/// A document holding no card articles writes as one slide.
	#[beet_core::test]
	fn writes_a_document_as_one_slide() {
		let mut world = formats(false);
		let root = world.spawn(rsx! { <h2>"Notes"</h2><p>"words"</p> }).id();
		let written = write(&mut world, root, default());
		let (mut again, again_root) =
			parse_in(formats(false), &written).unwrap();
		html(&mut again, again_root).xpect_eq(
			"<article data-card><div class=\"card-content\"><header class=\"card-heading\"><h1>Notes</h1></header><div class=\"card-columns\"><div class=\"card-column\"><p>words</p></div></div></div></article>",
		);
	}

	/// LibreOffice converts a written deck headless, the proxy for opening
	/// without a repair prompt, every slide's text and notes intact. Skips
	/// where `soffice` is absent.
	#[cfg(not(target_arch = "wasm32"))]
	#[beet_core::test]
	fn libreoffice_reads_a_written_deck() {
		if !has_soffice() {
			return;
		}
		let (mut world, root) = parse(&covering()).unwrap();
		let written = write(&mut world, root, default());
		soffice_convert(&written, "covering.pptx", "fodp")
			.xpect_contains("Field notes")
			.xpect_contains("Two columns")
			.xpect_contains("nested")
			.xpect_contains("A plain slide")
			.xpect_contains("Stall")
			.xpect_contains("First point.");
	}
}
