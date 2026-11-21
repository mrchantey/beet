//! [`WordRenderer`], any document written as a Word file, by the
//! [module docs](super)' Word mapping.
use super::office_picture::*;
use super::office_style::*;
use super::word_parser::WordHighlight;
use super::xml_part::XmlPart;
use crate::prelude::*;
use crate::style::*;
use beet_core::prelude::*;
use beet_net::prelude::RequestParts;

/// Writes any document as a Word file from the tree alone: its blocks in
/// order through the styles the format names, an A4 page, its lists through
/// one bullet and one decimal definition, its notes, pictures, links and
/// text boxes, and every look the cascade resolves in the `Print` medium on
/// a 602 px page, the root's as the document's defaults and each named
/// style's as its tag resolves. A run, paragraph or cell writes as direct
/// formatting only what it resolves differently from the structure a reader
/// reads it back into, so a written file read again resolves the same look
/// through the same rules. See the [module docs](super) for what a write can
/// say.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct WordRenderer;

/// A probe chain: the elements a reader reads a piece of a written file
/// back into, from the root down.
type Chain = Vec<ProbeTag>;

/// The named paragraph styles a write uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleName {
	Normal,
	Heading(u8),
	Quote,
	Preformatted,
	Caption,
	ListItem,
	FootnoteText,
	EndnoteText,
}

/// A block as the writer gathers it from the tree.
#[derive(Debug)]
enum BlockWrite {
	Paragraph(ParagraphWrite),
	Table(TableWrite),
	/// An `hr`, its colour its element's.
	Rule(Entity),
	PageBreak,
	/// A page break when the element resolves `break-after: page`.
	BreakAfter(Entity),
	/// An `aside`, a text box of its blocks.
	TextBox(Vec<BlockWrite>),
}

#[derive(Debug)]
struct ParagraphWrite {
	/// The element whose look is the paragraph's.
	entity: Entity,
	style: StyleName,
	/// What a reader reads it back into.
	chain: Chain,
	/// Its list's `w:num` and its level.
	numbering: Option<(usize, u8)>,
	/// The bookmark it opens, its block's `id`.
	id: Option<String>,
	inline: Vec<InlineWrite>,
	/// Whether it is a cell's lone paragraph, its shading the cell's.
	cell: bool,
}

/// The structure around a run that the format writes as structure rather
/// than as a look, and the inline elements whose looks it resolves.
#[derive(Debug, Clone, Default)]
struct RunContext {
	/// The structural inline elements a reader reads back, outermost first.
	tags: Vec<ProbeTag>,
	/// Every inline element around it, outermost first.
	ancestors: Vec<Entity>,
	code: bool,
	link: bool,
	mark: bool,
	vertical: Option<&'static str>,
	deleted: bool,
	/// Inside a `pre`, its newlines its lines.
	preformatted: bool,
}

#[derive(Debug)]
enum InlineWrite {
	Text {
		text: String,
		run: RunContext,
		/// The element its look resolves at, its text's parent.
		look: Entity,
	},
	Break {
		run: RunContext,
		look: Entity,
	},
	Image {
		entity: Entity,
	},
	Checkbox {
		checked: bool,
	},
	NoteRef {
		endnote: bool,
		number: usize,
	},
	Link {
		href: String,
		children: Vec<InlineWrite>,
	},
	Revision {
		inserted: bool,
		datetime: Option<String>,
		children: Vec<InlineWrite>,
	},
}

#[derive(Debug)]
struct TableWrite {
	id: Option<String>,
	caption: Option<ParagraphWrite>,
	columns: u32,
	rows: Vec<RowWrite>,
}

#[derive(Debug)]
struct RowWrite {
	header: bool,
	cells: Vec<CellWrite>,
}

#[derive(Debug)]
struct CellWrite {
	/// The `td` or `th`, none for a continuation of a merge from above.
	entity: Option<Entity>,
	/// What a reader reads the cell back into.
	chain: Chain,
	span: UVec2,
	/// Whether it starts a merge down.
	restart: bool,
	blocks: Vec<BlockWrite>,
}

/// Where a gathered block sits, so its paragraphs take the right style and
/// probe chain.
#[derive(Debug, Clone, Default)]
struct BlockContext {
	chain: Chain,
	quote: bool,
	/// A note's text, an endnote when `true`.
	note: Option<bool>,
}

/// A document as gathered for writing.
#[derive(Debug, Default)]
struct GatheredDocument {
	header: Vec<BlockWrite>,
	body: Vec<BlockWrite>,
	footer: Vec<BlockWrite>,
	/// Each referenced note in reference order, an endnote when `true`.
	notes: Vec<(bool, Vec<BlockWrite>)>,
	/// Each list's `w:num` in order, whether it counts.
	nums: Vec<bool>,
}

/// What a gather tracks as it walks.
#[derive(Debug, Default)]
struct GatherState {
	/// Each note definition by its `id`, and whether it is an endnote.
	definitions: HashMap<String, (Entity, bool)>,
	/// The number each referenced note writes as, by its definition's `id`.
	numbers: HashMap<String, usize>,
	/// The referenced footnotes and endnotes in reference order.
	footnotes: Vec<String>,
	endnotes: Vec<String>,
	/// Each list's `w:num`, whether it counts.
	nums: Vec<bool>,
	/// A container's `id`, for the next block gathered.
	pending_id: Option<String>,
}

/// The tree read for writing.
#[derive(SystemParam)]
struct WordGather<'w, 's> {
	children: Query<'w, 's, &'static Children>,
	elements: ElementQuery<'w, 's>,
	values: Query<'w, 's, &'static Value, Without<Element>>,
	tags: Query<'w, 's, &'static Element>,
	grid: TableGrid<'w, 's>,
}

/// Everything a write resolves before it writes.
struct Resolved {
	styles: OfficeStyleSheet,
	/// Each gathered element's computed style.
	computed: HashMap<Entity, ComputedStyle>,
	/// Each probe chain's computed styles, one per element.
	probes: HashMap<Chain, Vec<ComputedStyle>>,
	/// The elements resolving `break-after: page`.
	breaks: HashSet<Entity>,
	/// Each `img`'s `src`, `alt`, `width` and `height` attributes.
	images: HashMap<Entity, (String, String, Vec2)>,
}

/// The write in progress: the package and the counters its parts share.
struct WordWrite<'a> {
	resolved: &'a Resolved,
	package: OpcPackage,
	/// The next drawing's `wp:docPr` id.
	drawings: u32,
	/// The next bookmark's and revision's id.
	marks: u32,
	images: u32,
	/// A revision's author.
	author: String,
	/// What the next paragraph written opens with, a note's reference mark.
	lead: Option<String>,
}

impl WordRenderer {
	const DOCUMENT: &str = "word/document.xml";
	const DOCUMENT_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml";
	const STYLES_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml";
	const SETTINGS_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.settings+xml";
	const NUMBERING_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.numbering+xml";
	const FOOTNOTES_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.footnotes+xml";
	const ENDNOTES_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.endnotes+xml";
	const HEADER_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";
	const FOOTER_TYPE: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.footer+xml";
	const THEME_TYPE: &str =
		"application/vnd.openxmlformats-officedocument.theme+xml";
	/// The page's content box in px: A4 with one inch margins.
	const CONTENT: Vec2 = Vec2::new(602., 931.);
	/// The page's content width in twips, the content box's.
	const CONTENT_TWIPS: i64 = 9026;
	/// The font every run writes in, a code run's in [`MONOSPACE`](Self::MONOSPACE).
	const FONT: &str = "Calibri";
	const MONOSPACE: &str = "Courier New";
	/// The tags a block walk handles as blocks, every other as inline.
	const BLOCKS: [&str; 33] = [
		"p",
		"h1",
		"h2",
		"h3",
		"h4",
		"h5",
		"h6",
		"blockquote",
		"pre",
		"ul",
		"ol",
		"li",
		"table",
		"hr",
		"div",
		"section",
		"article",
		"main",
		"nav",
		"aside",
		"header",
		"footer",
		"figure",
		"figcaption",
		"dl",
		"dt",
		"dd",
		"details",
		"summary",
		"form",
		"fieldset",
		"address",
		"hgroup",
	];
	/// The namespaces a written body binds.
	const NAMESPACES: [(&str, &str); 10] = [
		("xmlns:w", OoxmlNamespace::WORD),
		("xmlns:r", OoxmlNamespace::RELATIONSHIPS),
		("xmlns:wp", OoxmlNamespace::WORD_DRAWING),
		("xmlns:a", OoxmlNamespace::DRAWING),
		(
			"xmlns:pic",
			"http://schemas.openxmlformats.org/drawingml/2006/picture",
		),
		(
			"xmlns:wps",
			"http://schemas.microsoft.com/office/word/2010/wordprocessingShape",
		),
		("xmlns:mc", OoxmlNamespace::COMPATIBILITY),
		("xmlns:w14", OoxmlNamespace::WORD_2010),
		(
			"xmlns:wp14",
			"http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing",
		),
		("mc:Ignorable", "w14 wp14"),
	];

	/// Writes `root` as a Word file's bytes.
	fn write(
		world: &mut World,
		root: Entity,
		request: &RequestParts,
	) -> Result<Vec<u8>> {
		let mut media = MediaContext::new(Medium::Print).with_viewport(
			MediaViewport::new(Self::CONTENT.x, Self::CONTENT.y),
		);
		if let Some(scheme) = OfficeStyleSheet::scheme(world, request) {
			media = media.with_scheme(scheme);
		}
		let styles = OfficeStyleSheet::resolve(world, root, media)?;
		// gather the header, body, footer and notes from the tree alone
		let document =
			world.with_state::<WordGather, _>(|gather| gather.document(root));
		let resolved = Resolved::new(world, styles, &document)?;
		let author = world
			.entity(root)
			.get::<DocMeta>()
			.and_then(|meta| meta.authors.first().cloned())
			.map(|author| author.to_string())
			.unwrap_or_else(|| "beet".into());
		let mut write = WordWrite {
			resolved: &resolved,
			package: OpcPackage::default(),
			drawings: 1,
			marks: 1,
			images: 0,
			author,
			lead: None,
		};
		write.write_document(&document)?;
		let mut package = write.package;
		if let Some(meta) = world.entity(root).get::<DocMeta>() {
			meta.write_package(&mut package);
		}
		package.to_bytes()
	}
}

impl WordGather<'_, '_> {
	/// The header, body, footer and notes of `root`: its first child when a
	/// `header`, its last when a `footer`, every note definition, and the
	/// blocks of the rest.
	fn document(&self, root: Entity) -> GatheredDocument {
		let mut state = GatherState::default();
		for entity in self.descendants(root) {
			let Ok(view) = self.elements.get(entity) else {
				continue;
			};
			if view.tag() == "div"
				&& view.contains_class("footnote-definition")
				&& let Some(id) = view.attribute("id")
			{
				let id = id.value.to_string();
				let endnote = id.starts_with("en-");
				state.definitions.insert(id, (entity, endnote));
			}
		}
		let children = self.children_of(root);
		let tag = |entity: Option<&Entity>| {
			entity.and_then(|entity| self.tag(*entity))
		};
		let header =
			(tag(children.first()) == Some("header")).then(|| children[0]);
		let footer = (children.len() > usize::from(header.is_some())
			&& tag(children.last()) == Some("footer"))
		.then(|| children[children.len() - 1]);
		let context = BlockContext::default();
		let part =
			|entity: Option<Entity>, chain: &str, state: &mut GatherState| {
				let mut blocks = Vec::new();
				if let Some(entity) = entity {
					let context = BlockContext {
						chain: vec![ProbeTag::new(chain)],
						..default()
					};
					self.container(entity, &context, state, &mut blocks);
				}
				blocks
			};
		let header_blocks = part(header, "header", &mut state);
		let footer_blocks = part(footer, "footer", &mut state);
		let mut body = Vec::new();
		let mut pending = Vec::new();
		for child in children
			.iter()
			.copied()
			.filter(|child| Some(*child) != header && Some(*child) != footer)
		{
			self.child(
				root,
				child,
				&context,
				&mut state,
				&mut body,
				&mut pending,
			);
		}
		self.flush(root, &context, &mut state, &mut body, &mut pending);
		// a note may reference another, so the list grows as it is written
		let mut notes = Vec::new();
		for endnote in [false, true] {
			let mut index = 0;
			loop {
				let list = match endnote {
					false => &state.footnotes,
					true => &state.endnotes,
				};
				let Some(id) = list.get(index).cloned() else {
					break;
				};
				index += 1;
				let Some((entity, _)) = state.definitions.get(&id).copied()
				else {
					continue;
				};
				let context = BlockContext {
					chain: vec![ProbeTag::with_class(
						"div",
						"footnote-definition",
					)],
					note: Some(endnote),
					..default()
				};
				let mut blocks = Vec::new();
				self.container(entity, &context, &mut state, &mut blocks);
				notes.push((endnote, blocks));
			}
		}
		GatheredDocument {
			header: header_blocks,
			body,
			footer: footer_blocks,
			notes,
			nums: state.nums,
		}
	}

	fn children_of(&self, entity: Entity) -> Vec<Entity> {
		self.children
			.get(entity)
			.map(|children| children.to_vec())
			.unwrap_or_default()
	}

	fn descendants(&self, entity: Entity) -> Vec<Entity> {
		self.children.iter_descendants_depth_first(entity).collect()
	}

	fn tag(&self, entity: Entity) -> Option<&str> {
		self.tags.get(entity).ok().map(|element| element.tag())
	}

	fn attribute(&self, entity: Entity, key: &str) -> Option<String> {
		self.elements
			.get(entity)
			.ok()?
			.attribute(key)
			.map(|attribute| attribute.value.to_string())
	}

	/// Whether `entity` is a block: an element of a block tag, beet's page
	/// break among them.
	fn is_block(&self, entity: Entity) -> bool {
		self.tag(entity)
			.is_some_and(|tag| WordRenderer::BLOCKS.contains(&tag))
	}

	/// Gathers the children of `entity` as blocks, its inline content as
	/// paragraphs of its own look.
	fn container(
		&self,
		entity: Entity,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
	) {
		let mut pending = Vec::new();
		for child in self.children_of(entity) {
			self.child(entity, child, context, state, out, &mut pending);
		}
		self.flush(entity, context, state, out, &mut pending);
	}

	/// Gathers `child` of `parent`: a block in its place, an inline node
	/// into `pending`, the paragraph of `parent`'s inline content.
	fn child(
		&self,
		parent: Entity,
		child: Entity,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
		pending: &mut Vec<Entity>,
	) {
		if !self.is_block(child) {
			pending.push(child);
			return;
		}
		self.flush(parent, context, state, out, pending);
		self.block(child, context, state, out);
	}

	/// The paragraph of the inline nodes `pending` holds, of `parent`'s look.
	fn flush(
		&self,
		parent: Entity,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
		pending: &mut Vec<Entity>,
	) {
		if pending.is_empty() {
			return;
		}
		let nodes = core::mem::take(pending);
		// whitespace between blocks is no paragraph
		let blank = nodes.iter().all(|node| {
			self.values
				.get(*node)
				.is_ok_and(|value| value.to_string().trim().is_empty())
		});
		if blank {
			return;
		}
		let paragraph = self.paragraph(parent, &nodes, context, state);
		out.push(BlockWrite::Paragraph(paragraph));
	}

	/// A paragraph of `nodes`, its look `entity`'s, styled by its tag and
	/// where it sits.
	fn paragraph(
		&self,
		entity: Entity,
		nodes: &[Entity],
		context: &BlockContext,
		state: &mut GatherState,
	) -> ParagraphWrite {
		let tag = self.tag(entity).unwrap_or("p");
		let heading = DocumentText::heading_level(tag);
		let (style, leaf) = match (heading, context.note, context.quote) {
			(Some(level), ..) => {
				(StyleName::Heading(level), format!("h{level}"))
			}
			(None, Some(false), _) => (StyleName::FootnoteText, "p".into()),
			(None, Some(true), _) => (StyleName::EndnoteText, "p".into()),
			(None, None, true) => (StyleName::Quote, "p".into()),
			(None, None, false) => (StyleName::Normal, "p".into()),
		};
		let mut chain = context.chain.clone();
		chain.push(ProbeTag::new(leaf));
		let mut inline = Vec::new();
		for node in nodes {
			self.inline(
				*node,
				entity,
				&RunContext::default(),
				state,
				&mut inline,
			);
		}
		ParagraphWrite {
			entity,
			style,
			chain,
			numbering: None,
			id: self.take_id(entity, state),
			inline,
			cell: false,
		}
	}

	/// The bookmark a block opens: its own `id`, else a container's held
	/// for it, which waits for the next block when the block has its own.
	fn take_id(
		&self,
		entity: Entity,
		state: &mut GatherState,
	) -> Option<String> {
		match (self.attribute(entity, "id"), state.pending_id.take()) {
			(Some(own), Some(pending)) => {
				if pending != own {
					state.pending_id = Some(pending);
				}
				Some(own)
			}
			(own, pending) => own.or(pending),
		}
	}

	/// Gathers one block element.
	fn block(
		&self,
		entity: Entity,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
	) {
		let Ok(view) = self.elements.get(entity) else {
			return;
		};
		let tag = view.tag().to_string();
		match tag.as_str() {
			"div"
				if view.contains_class("footnote-definition")
					&& view.attribute("id").is_some() =>
			{
				return;
			}
			"div" if view.contains_class("page-break") => {
				out.push(BlockWrite::PageBreak);
				return;
			}
			"hr" => out.push(BlockWrite::Rule(entity)),
			"pre" => self.preformatted(entity, context, state, out),
			"ul" | "ol" => {
				let ordered = tag == "ol";
				state.nums.push(ordered);
				let num = state.nums.len();
				self.list(entity, ordered, num, 0, context, state, out);
			}
			"table" => {
				let table = self.table(entity, context, state);
				out.push(BlockWrite::Table(table));
			}
			"aside" => {
				let context = BlockContext {
					chain: vec![ProbeTag::new("aside")],
					..default()
				};
				let mut blocks = Vec::new();
				self.container(entity, &context, state, &mut blocks);
				out.push(BlockWrite::TextBox(blocks));
			}
			"blockquote" => {
				let mut context = context.clone();
				context.chain.push(ProbeTag::new("blockquote"));
				context.quote = true;
				self.with_id(entity, state);
				self.container(entity, &context, state, out);
			}
			tag if tag == "p" || DocumentText::heading_level(tag).is_some() => {
				let nodes = self.children_of(entity);
				// a paragraph holding blocks is a container of them
				match nodes.iter().any(|node| self.is_block(*node)) {
					true => {
						self.with_id(entity, state);
						self.container(entity, context, state, out);
					}
					false => {
						let paragraph =
							self.paragraph(entity, &nodes, context, state);
						out.push(BlockWrite::Paragraph(paragraph));
					}
				}
			}
			// any other block groups its blocks, which write in order
			_ => {
				self.with_id(entity, state);
				self.container(entity, context, state, out);
			}
		}
		out.push(BlockWrite::BreakAfter(entity));
	}

	/// Holds a container's `id` for the next block gathered.
	fn with_id(&self, entity: Entity, state: &mut GatherState) {
		if state.pending_id.is_none() {
			state.pending_id = self.attribute(entity, "id");
		}
	}

	/// A `pre`, a preformatted paragraph per line.
	fn preformatted(
		&self,
		entity: Entity,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
	) {
		let run = RunContext {
			preformatted: true,
			..default()
		};
		let mut inline = Vec::new();
		for child in self.children_of(entity) {
			self.inline(child, entity, &run, state, &mut inline);
		}
		let mut chain = context.chain.clone();
		chain.push(ProbeTag::new("pre"));
		let mut id = self.take_id(entity, state);
		for line in InlineWrite::lines(inline) {
			out.push(BlockWrite::Paragraph(ParagraphWrite {
				entity,
				style: StyleName::Preformatted,
				chain: chain.clone(),
				numbering: None,
				id: id.take(),
				inline: line,
				cell: false,
			}));
		}
	}

	/// A list's items at `level` of the list `num`, each nested list at the
	/// level below, a nested list of the other kind its own list.
	#[allow(clippy::too_many_arguments)]
	fn list(
		&self,
		list: Entity,
		ordered: bool,
		num: usize,
		level: u8,
		context: &BlockContext,
		state: &mut GatherState,
		out: &mut Vec<BlockWrite>,
	) {
		let mut list_context = context.clone();
		list_context.chain.push(ProbeTag::new(if ordered {
			"ol"
		} else {
			"ul"
		}));
		self.with_id(list, state);
		for item in self.children_of(list) {
			if self.tag(item) != Some("li") {
				continue;
			}
			let mut item_context = list_context.clone();
			item_context.chain.push(ProbeTag::new("li"));
			// the item's paragraph: its inline content up to its first block,
			// else its first block's when that is a paragraph or a `div` of
			// inline content, as a loose item or one holding a nested list
			// wraps its words
			let children = self.children_of(item);
			let first_block =
				children.iter().position(|child| self.is_block(*child));
			let leading = &children[..first_block.unwrap_or(children.len())];
			let loose = first_block.filter(|index| {
				let block = children[*index];
				matches!(self.tag(block), Some("p" | "div"))
					&& !self
						.children_of(block)
						.iter()
						.any(|node| self.is_block(*node))
					&& leading.iter().all(|node| {
						self.values.get(*node).is_ok_and(|value| {
							value.to_string().trim().is_empty()
						})
					})
			});
			let (look, nodes, rest) = match loose {
				Some(index) => (
					children[index],
					self.children_of(children[index]),
					&children[index + 1..],
				),
				None => (item, leading.to_vec(), &children[leading.len()..]),
			};
			let mut inline = Vec::new();
			for node in nodes {
				self.inline(
					node,
					look,
					&RunContext::default(),
					state,
					&mut inline,
				);
			}
			out.push(BlockWrite::Paragraph(ParagraphWrite {
				entity: look,
				style: StyleName::ListItem,
				chain: item_context.chain.clone(),
				numbering: Some((num, level)),
				id: self.take_id(item, state),
				inline,
				cell: false,
			}));
			// what follows writes after it, a nested list a level below
			let mut pending = Vec::new();
			for child in rest.iter().copied() {
				match self.tag(child) {
					Some(tag @ ("ul" | "ol")) => {
						self.flush(
							item,
							&item_context,
							state,
							out,
							&mut pending,
						);
						let nested = tag == "ol";
						let nested_num = match nested == ordered {
							true => num,
							false => {
								state.nums.push(nested);
								state.nums.len()
							}
						};
						self.list(
							child,
							nested,
							nested_num,
							level + 1,
							&item_context,
							state,
							out,
						);
					}
					_ => self.child(
						item,
						child,
						&item_context,
						state,
						out,
						&mut pending,
					),
				}
			}
			self.flush(item, &item_context, state, out, &mut pending);
		}
	}

	/// A table by its grid: its caption, its rows, a merge's first cell
	/// spanning and the slots it covers below continuations.
	fn table(
		&self,
		table: Entity,
		context: &BlockContext,
		state: &mut GatherState,
	) -> TableWrite {
		let id = self.take_id(table, state);
		let mut table_chain = context.chain.clone();
		table_chain.push(ProbeTag::new("table"));
		let caption = self
			.children_of(table)
			.into_iter()
			.find(|child| self.tag(*child) == Some("caption"))
			.map(|caption| {
				let mut chain = table_chain.clone();
				chain.push(ProbeTag::new("caption"));
				let mut inline = Vec::new();
				for node in self.children_of(caption) {
					self.inline(
						node,
						caption,
						&RunContext::default(),
						state,
						&mut inline,
					);
				}
				ParagraphWrite {
					entity: caption,
					style: StyleName::Caption,
					chain,
					numbering: None,
					id: self.attribute(caption, "id"),
					inline,
					cell: false,
				}
			});
		let (rows, cells) = (self.grid.rows(table), self.grid.cells(table));
		let head = self
			.children_of(table)
			.into_iter()
			.filter(|child| self.tag(*child) == Some("thead"))
			.flat_map(|head| self.children_of(head))
			.collect::<Vec<_>>();
		let columns = cells
			.iter()
			.map(|cell| cell.start.x + cell.span.x)
			.max()
			.unwrap_or(1)
			.max(1);
		let mut written = Vec::new();
		for (index, row) in rows.iter().enumerate() {
			let header = head.contains(row);
			let group = if header { "thead" } else { "tbody" };
			let y = index as u32;
			let mut row_cells = cells
				.iter()
				.filter(|cell| cell.start.y == y)
				.map(|cell| {
					let tag = self.tag(cell.entity).unwrap_or("td").to_string();
					let mut chain = table_chain.clone();
					chain.extend([
						ProbeTag::new(group),
						ProbeTag::new("tr"),
						ProbeTag::new(tag),
					]);
					let context = BlockContext {
						chain: chain.clone(),
						..default()
					};
					let mut blocks = Vec::new();
					self.container(cell.entity, &context, state, &mut blocks);
					// a lone plain paragraph reads back as the cell's content
					if let [BlockWrite::Paragraph(paragraph)] =
						blocks.as_mut_slice()
						&& paragraph.style == StyleName::Normal
					{
						paragraph.chain = chain.clone();
						paragraph.cell = true;
					}
					(cell.start.x, CellWrite {
						entity: Some(cell.entity),
						chain,
						span: cell.span,
						restart: cell.span.y > 1,
						blocks,
					})
				})
				.collect::<Vec<_>>();
			// the slots a merge from above covers continue it
			for cell in cells
				.iter()
				.filter(|cell| cell.start.y < y && cell.last().y >= y)
			{
				row_cells.push((cell.start.x, CellWrite {
					entity: None,
					chain: Vec::new(),
					span: UVec2::new(cell.span.x, 1),
					restart: false,
					blocks: Vec::new(),
				}));
			}
			row_cells.sort_by_key(|(start, _)| *start);
			written.push(RowWrite {
				header,
				cells: row_cells.into_iter().map(|(_, cell)| cell).collect(),
			});
		}
		TableWrite {
			id,
			caption,
			columns,
			rows: written,
		}
	}

	/// Gathers the inline node `entity` of the block `block` inside `run`.
	fn inline(
		&self,
		entity: Entity,
		block: Entity,
		run: &RunContext,
		state: &mut GatherState,
		out: &mut Vec<InlineWrite>,
	) {
		if let Ok(value) = self.values.get(entity) {
			let text = value.to_string();
			if !text.is_empty() {
				out.push(InlineWrite::Text {
					// a newline outside `pre` is a space, as HTML lays it out
					text: match run.preformatted {
						true => text,
						false => text.replace('\n', " "),
					},
					run: run.clone(),
					look: run.ancestors.last().copied().unwrap_or(block),
				});
			}
			return;
		}
		let Ok(view) = self.elements.get(entity) else {
			for child in self.children_of(entity) {
				self.inline(child, block, run, state, out);
			}
			return;
		};
		let tag = view.tag().to_string();
		// metadata is no text, and an inline `svg` a picture no file holds
		if RenderTreeQuery::METADATA_TAGS.contains(&tag.as_str())
			|| tag == "svg"
		{
			return;
		}
		let mut inner = run.clone();
		inner.ancestors.push(entity);
		let push_tag = |inner: &mut RunContext, tag: &str| {
			inner.tags.push(ProbeTag::new(tag))
		};
		match tag.as_str() {
			"br" => {
				out.push(InlineWrite::Break {
					run: run.clone(),
					look: run.ancestors.last().copied().unwrap_or(block),
				});
				return;
			}
			"img" => {
				out.push(InlineWrite::Image { entity });
				return;
			}
			"input" => {
				match view.checkbox() {
					Some(checked) => {
						out.push(InlineWrite::Checkbox { checked })
					}
					// a form control writes its value
					None => {
						let value = view.attribute_string("value");
						if !value.is_empty() {
							out.push(InlineWrite::Text {
								text: value,
								run: run.clone(),
								look: entity,
							});
						}
					}
				}
				return;
			}
			"sup" => {
				if let Some(reference) = self.note_reference(entity, state) {
					out.push(reference);
					return;
				}
				push_tag(&mut inner, "sup");
				inner.vertical = Some("superscript");
			}
			"sub" => {
				push_tag(&mut inner, "sub");
				inner.vertical = Some("subscript");
			}
			"code" | "kbd" | "samp" => {
				push_tag(&mut inner, "code");
				inner.code = true;
			}
			"mark" => {
				push_tag(&mut inner, "mark");
				inner.mark = true;
			}
			"a" => {
				if let Some(href) =
					view.attribute("href").map(|href| href.value.to_string())
				{
					push_tag(&mut inner, "a");
					inner.link = true;
					let mut children = Vec::new();
					for child in self.children_of(entity) {
						self.inline(child, block, &inner, state, &mut children);
					}
					out.push(InlineWrite::Link { href, children });
					return;
				}
			}
			"ins" | "del" => {
				push_tag(&mut inner, &tag);
				inner.deleted = tag == "del";
				let mut children = Vec::new();
				for child in self.children_of(entity) {
					self.inline(child, block, &inner, state, &mut children);
				}
				out.push(InlineWrite::Revision {
					inserted: tag == "ins",
					datetime: view
						.attribute("datetime")
						.map(|date| date.value.to_string()),
					children,
				});
				return;
			}
			_ => {}
		}
		for child in self.children_of(entity) {
			self.inline(child, block, &inner, state, out);
		}
	}

	/// A `sup` holding only a link to a note definition, as its reference,
	/// numbered on its first.
	fn note_reference(
		&self,
		sup: Entity,
		state: &mut GatherState,
	) -> Option<InlineWrite> {
		let elements = self
			.children_of(sup)
			.into_iter()
			.filter(|child| {
				self.values
					.get(*child)
					.map_or(true, |value| !value.to_string().trim().is_empty())
			})
			.collect::<Vec<_>>();
		let [link] = elements.as_slice() else {
			return None;
		};
		(self.tag(*link) == Some("a")).then_some(())?;
		let id = self
			.attribute(*link, "href")?
			.strip_prefix('#')?
			.to_string();
		let (_, endnote) = *state.definitions.get(&id)?;
		let number = match state.numbers.get(&id) {
			Some(number) => *number,
			None => {
				let list = match endnote {
					false => &mut state.footnotes,
					true => &mut state.endnotes,
				};
				list.push(id.clone());
				state.numbers.insert(id, list.len());
				list.len()
			}
		};
		Some(InlineWrite::NoteRef { endnote, number })
	}
}

impl InlineWrite {
	/// `inline` split at each newline its top level texts hold, a newline
	/// ending the last line making no line of its own.
	fn lines(inline: Vec<InlineWrite>) -> Vec<Vec<InlineWrite>> {
		let mut lines = vec![Vec::new()];
		for item in inline {
			match item {
				InlineWrite::Text { text, run, look } => {
					let mut parts = text.split('\n').peekable();
					while let Some(part) = parts.next() {
						if !part.is_empty() {
							lines.last_mut().unwrap().push(InlineWrite::Text {
								text: part.to_string(),
								run: run.clone(),
								look,
							});
						}
						if parts.peek().is_some() {
							lines.push(Vec::new());
						}
					}
				}
				other => lines.last_mut().unwrap().push(other),
			}
		}
		if lines.len() > 1 && lines.last().is_some_and(Vec::is_empty) {
			lines.pop();
		}
		lines
	}
}

impl GatheredDocument {
	/// The blocks of every part: header, body, footer and each note.
	fn parts(&self) -> impl Iterator<Item = &[BlockWrite]> {
		[&self.header, &self.body, &self.footer]
			.into_iter()
			.map(Vec::as_slice)
			.chain(self.notes.iter().map(|(_, blocks)| blocks.as_slice()))
	}
}

impl BlockWrite {
	/// Every element and probe chain `blocks` resolve looks for.
	fn collect(
		blocks: &[BlockWrite],
		entities: &mut Vec<Entity>,
		chains: &mut Vec<Chain>,
		breaks: &mut Vec<Entity>,
	) {
		for block in blocks {
			match block {
				BlockWrite::Paragraph(paragraph) => {
					paragraph.collect(entities, chains)
				}
				BlockWrite::Table(table) => {
					if let Some(caption) = &table.caption {
						caption.collect(entities, chains);
					}
					for cell in table.rows.iter().flat_map(|row| &row.cells) {
						if let Some(entity) = cell.entity {
							entities.push(entity);
							chains.push(cell.chain.clone());
						}
						Self::collect(&cell.blocks, entities, chains, breaks);
					}
				}
				BlockWrite::Rule(entity) => entities.push(*entity),
				BlockWrite::BreakAfter(entity) => breaks.push(*entity),
				BlockWrite::TextBox(blocks) => {
					Self::collect(blocks, entities, chains, breaks)
				}
				BlockWrite::PageBreak => {}
			}
		}
	}
}

impl ParagraphWrite {
	fn collect(&self, entities: &mut Vec<Entity>, chains: &mut Vec<Chain>) {
		entities.push(self.entity);
		chains.push(self.chain.clone());
		InlineWrite::collect(&self.inline, &self.chain, entities, chains);
	}
}

impl InlineWrite {
	fn collect(
		inline: &[InlineWrite],
		chain: &Chain,
		entities: &mut Vec<Entity>,
		chains: &mut Vec<Chain>,
	) {
		for item in inline {
			match item {
				InlineWrite::Text { run, look, .. }
				| InlineWrite::Break { run, look } => {
					entities.push(*look);
					entities.extend(&run.ancestors);
					chains
						.push(chain.iter().chain(&run.tags).cloned().collect());
				}
				InlineWrite::Image { entity } => entities.push(*entity),
				InlineWrite::Link { children, .. }
				| InlineWrite::Revision { children, .. } => {
					Self::collect(children, chain, entities, chains)
				}
				InlineWrite::Checkbox { .. } | InlineWrite::NoteRef { .. } => {}
			}
		}
	}
}

impl Resolved {
	/// The probe chains every named style writes from.
	fn named_chains() -> Vec<Chain> {
		let chain = |tags: &[&str]| {
			tags.iter()
				.map(|tag| ProbeTag::new(*tag))
				.collect::<Chain>()
		};
		let mut chains = vec![
			chain(&["p"]),
			chain(&["blockquote", "p"]),
			chain(&["pre"]),
			chain(&["p", "code"]),
			chain(&["p", "a"]),
			chain(&["p", "mark"]),
			chain(&["table", "caption"]),
			chain(&["ul", "li"]),
			vec![
				ProbeTag::with_class("div", "footnote-definition"),
				ProbeTag::new("p"),
			],
		];
		chains.extend((1..=6).map(|level| chain(&[&format!("h{level}")])));
		chains
	}

	/// Resolves every look the gathered blocks write.
	fn new(
		world: &mut World,
		styles: OfficeStyleSheet,
		document: &GatheredDocument,
	) -> Result<Self> {
		let (mut entities, mut chains, mut breaks) =
			(Vec::new(), Self::named_chains(), Vec::new());
		for blocks in document.parts() {
			BlockWrite::collect(
				blocks,
				&mut entities,
				&mut chains,
				&mut breaks,
			);
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
		let breaks = breaks
			.iter()
			.copied()
			.zip(styles.breaks_after(world, &breaks)?)
			.filter_map(|(entity, breaks)| breaks.then_some(entity))
			.collect();
		let images = world.with_state::<ElementQuery, _>(|elements| {
			entities
				.iter()
				.filter_map(|entity| {
					let view = elements.get(*entity).ok()?;
					(view.tag() == "img").then(|| {
						let side = |key: &str| {
							view.attribute(key)
								.and_then(|value| {
									value
										.value
										.to_string()
										.trim_end_matches("px")
										.parse::<f32>()
										.ok()
								})
								.unwrap_or(0.)
						};
						(
							*entity,
							(
								view.attribute_string("src"),
								view.attribute_string("alt"),
								Vec2::new(side("width"), side("height")),
							),
						)
					})
				})
				.collect()
		});
		Self {
			styles,
			computed,
			probes,
			breaks,
			images,
		}
		.xok()
	}

	fn computed(&self, entity: Entity) -> ComputedStyle {
		self.computed.get(&entity).cloned().unwrap_or_default()
	}

	/// The styles of each element of `chain`.
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

	/// A run's look: its element's, its background the nearest of its inline
	/// ancestors'.
	fn run_look(&self, look: Entity, run: &RunContext) -> OfficeLook {
		let mut out = OfficeLook::from_computed(&self.computed(look));
		out.background =
			run.ancestors.iter().rev().find_map(|ancestor| {
				self.computed(*ancestor).visual.background
			});
		out
	}

	/// The look a reader reading a run back resolves with no look of its
	/// own: its paragraph's chain and its structural elements.
	fn run_base(&self, chain: &[ProbeTag], run: &RunContext) -> OfficeLook {
		let full = chain.iter().chain(&run.tags).cloned().collect::<Chain>();
		let styles = self.probe(&full);
		let mut out = styles
			.last()
			.map(OfficeLook::from_computed)
			.unwrap_or_default();
		out.background = styles
			.iter()
			.skip(chain.len())
			.rev()
			.find_map(|style| style.visual.background);
		out
	}

	/// A computed margin in px.
	fn px(&self, length: Length) -> f32 {
		OfficeUnits::px(
			length,
			WordRenderer::CONTENT.x,
			*self.styles.media().viewport(),
		)
	}
}

/// The run properties a write writes, in the order the schema requires.
#[derive(Debug, Default, Clone, PartialEq)]
struct RunProperties {
	style: Option<&'static str>,
	monospace: bool,
	bold: Option<bool>,
	italic: Option<bool>,
	strike: Option<bool>,
	color: Option<Color>,
	size_px: Option<f32>,
	highlight: Option<&'static str>,
	underline: Option<bool>,
	shading: Option<Color>,
	vertical: Option<&'static str>,
}

impl RunProperties {
	/// What `look` says differently from `base`, as run properties.
	fn diff(look: &OfficeLook, base: &OfficeLook) -> Self {
		let flag = |look: Option<bool>, base: Option<bool>| {
			(look.unwrap_or(false) != base.unwrap_or(false))
				.then(|| look.unwrap_or(false))
		};
		Self {
			bold: flag(look.bold, base.bold),
			italic: flag(look.italic, base.italic),
			strike: flag(look.strike, base.strike),
			underline: flag(look.underline, base.underline),
			color: (look.color != base.color).then_some(look.color).flatten(),
			size_px: look.font_size_px.filter(|size| {
				base.font_size_px
					.is_none_or(|base| (base - size).abs() > 0.01)
			}),
			shading: (look.background != base.background)
				.then_some(look.background)
				.flatten(),
			..default()
		}
	}

	/// The properties as an `rPr`, none when they say nothing.
	fn write(&self, styles: &OfficeStyleSheet, xml: &mut XmlPart) {
		if *self == Self::default() {
			return;
		}
		xml.open("w:rPr", &[]);
		self.write_inner(styles, xml);
		xml.close("w:rPr");
	}

	fn write_inner(&self, styles: &OfficeStyleSheet, xml: &mut XmlPart) {
		let toggle =
			|xml: &mut XmlPart, name: &str, value: Option<bool>| match value {
				Some(true) => {
					xml.empty(name, &[]);
				}
				Some(false) => {
					xml.empty(name, &[("w:val", "0")]);
				}
				None => {}
			};
		if let Some(style) = self.style {
			xml.empty("w:rStyle", &[("w:val", style)]);
		}
		if self.monospace {
			let font = WordRenderer::MONOSPACE;
			xml.empty("w:rFonts", &[
				("w:ascii", font),
				("w:hAnsi", font),
				("w:cs", font),
			]);
		}
		toggle(xml, "w:b", self.bold);
		toggle(xml, "w:i", self.italic);
		toggle(xml, "w:strike", self.strike);
		if let Some(color) = self.color {
			WordWrite::color(
				styles,
				xml,
				"w:color",
				"w:val",
				"w:themeColor",
				color,
			);
		}
		if let Some(size) = self.size_px {
			let half = OfficeUnits::half_points(size).to_string();
			xml.empty("w:sz", &[("w:val", &half)])
				.empty("w:szCs", &[("w:val", &half)]);
		}
		if let Some(highlight) = self.highlight {
			xml.empty("w:highlight", &[("w:val", highlight)]);
		}
		match self.underline {
			Some(true) => {
				xml.empty("w:u", &[("w:val", "single")]);
			}
			Some(false) => {
				xml.empty("w:u", &[("w:val", "none")]);
			}
			None => {}
		}
		if let Some(shading) = self.shading {
			WordWrite::shading(styles, xml, shading);
		}
		if let Some(vertical) = self.vertical {
			xml.empty("w:vertAlign", &[("w:val", vertical)]);
		}
	}
}

impl WordWrite<'_> {
	fn styles(&self) -> &OfficeStyleSheet { &self.resolved.styles }

	/// Writes a colour element `name`, its value and, where a slot holds the
	/// colour, the slot.
	fn color(
		styles: &OfficeStyleSheet,
		xml: &mut XmlPart,
		name: &str,
		value: &str,
		theme: &str,
		color: Color,
	) {
		let hex = OfficeColor::hex(color);
		match styles.theme_slot(color) {
			Some(slot) => {
				xml.empty(name, &[(value, &hex), (theme, slot.word_name())])
			}
			None => xml.empty(name, &[(value, &hex)]),
		};
	}

	/// Writes a solid shading of `color`.
	fn shading(styles: &OfficeStyleSheet, xml: &mut XmlPart, color: Color) {
		let hex = OfficeColor::hex(color);
		match styles.theme_slot(color) {
			Some(slot) => xml.empty("w:shd", &[
				("w:val", "clear"),
				("w:color", "auto"),
				("w:fill", &hex),
				("w:themeFill", slot.word_name()),
			]),
			None => xml.empty("w:shd", &[
				("w:val", "clear"),
				("w:color", "auto"),
				("w:fill", &hex),
			]),
		};
	}

	/// Writes every part: the document, its header and footer, notes,
	/// styles, numbering, settings and theme.
	fn write_document(&mut self, gathered: &GatheredDocument) -> Result {
		let (header, body, footer, notes) = (
			gathered.header.as_slice(),
			gathered.body.as_slice(),
			gathered.footer.as_slice(),
			gathered.notes.as_slice(),
		);
		let document = WordRenderer::DOCUMENT;
		let mut xml = XmlPart::new();
		xml.open("w:document", &WordRenderer::NAMESPACES);
		if let Some(background) = self.styles().defaults().background {
			Self::color(
				self.styles(),
				&mut xml,
				"w:background",
				"w:color",
				"w:themeColor",
				background,
			);
		}
		xml.open("w:body", &[]);
		self.blocks(document, body, &mut xml);
		// the section: its header and footer, an A4 page and one inch margins
		xml.open("w:sectPr", &[]);
		for (blocks, name, reference, kind, content_type) in [
			(
				header,
				"header",
				"w:headerReference",
				OpcRelationship::HEADER,
				WordRenderer::HEADER_TYPE,
			),
			(
				footer,
				"footer",
				"w:footerReference",
				OpcRelationship::FOOTER,
				WordRenderer::FOOTER_TYPE,
			),
		] {
			if blocks.is_empty() {
				continue;
			}
			let path = format!("word/{name}1.xml");
			let mut part = XmlPart::new();
			let root = if name == "header" { "w:hdr" } else { "w:ftr" };
			part.open(root, &WordRenderer::NAMESPACES);
			self.blocks(&path, blocks, &mut part);
			part.close(root);
			self.package
				.insert(path.as_str(), content_type, part.into_bytes());
			let id = self.package.relate(Some(document), kind, &path);
			xml.empty(reference, &[("w:type", "default"), ("r:id", &id)]);
		}
		xml.empty("w:pgSz", &[("w:w", "11906"), ("w:h", "16838")])
			.empty("w:pgMar", &[
				("w:top", "1440"),
				("w:right", "1440"),
				("w:bottom", "1440"),
				("w:left", "1440"),
				("w:header", "708"),
				("w:footer", "708"),
				("w:gutter", "0"),
			])
			.close("w:sectPr")
			.close("w:body")
			.close("w:document");
		self.package.insert(
			document,
			WordRenderer::DOCUMENT_TYPE,
			xml.into_bytes(),
		);
		self.package
			.relate(None, OpcRelationship::OFFICE_DOCUMENT, document);
		let has_notes = [false, true]
			.map(|endnote| notes.iter().any(|(kind, _)| *kind == endnote));
		for (endnote, written) in [false, true].into_iter().zip(has_notes) {
			if written {
				self.notes(endnote, notes);
			}
		}
		if !gathered.nums.is_empty() {
			self.package.insert(
				"word/numbering.xml",
				WordRenderer::NUMBERING_TYPE,
				Self::numbering_xml(&gathered.nums),
			);
			self.package.relate(
				Some(document),
				OpcRelationship::NUMBERING,
				"word/numbering.xml",
			);
		}
		self.package.insert(
			"word/styles.xml",
			WordRenderer::STYLES_TYPE,
			self.styles_xml(),
		);
		self.package.relate(
			Some(document),
			OpcRelationship::STYLES,
			"word/styles.xml",
		);
		self.package.insert(
			"word/settings.xml",
			WordRenderer::SETTINGS_TYPE,
			self.settings_xml(has_notes),
		);
		self.package.relate(
			Some(document),
			OpcRelationship::SETTINGS,
			"word/settings.xml",
		);
		self.package.insert(
			"word/theme/theme1.xml",
			WordRenderer::THEME_TYPE,
			self.styles().palette().theme_part(),
		);
		self.package.relate(
			Some(document),
			OpcRelationship::THEME,
			"word/theme/theme1.xml",
		);
		Ok(())
	}

	/// The notes part of `endnote`'s kind: Word's separator notes, then each
	/// note, its reference mark leading its first paragraph.
	fn notes(&mut self, endnote: bool, notes: &[(bool, Vec<BlockWrite>)]) {
		let (root, element, path, kind, content_type, reference) = match endnote
		{
			false => (
				"w:footnotes",
				"w:footnote",
				"word/footnotes.xml",
				OpcRelationship::FOOTNOTES,
				WordRenderer::FOOTNOTES_TYPE,
				"w:footnoteRef",
			),
			true => (
				"w:endnotes",
				"w:endnote",
				"word/endnotes.xml",
				OpcRelationship::ENDNOTES,
				WordRenderer::ENDNOTES_TYPE,
				"w:endnoteRef",
			),
		};
		let style = match endnote {
			false => "FootnoteReference",
			true => "EndnoteReference",
		};
		let mut xml = XmlPart::new();
		xml.open(root, &WordRenderer::NAMESPACES);
		for (id, separator) in
			[("-1", "separator"), ("0", "continuationSeparator")]
		{
			xml.open(element, &[("w:type", separator), ("w:id", id)])
				.open("w:p", &[])
				.open("w:pPr", &[])
				.empty("w:spacing", &[
					("w:after", "0"),
					("w:line", "240"),
					("w:lineRule", "auto"),
				])
				.close("w:pPr")
				.open("w:r", &[])
				.empty(&format!("w:{separator}"), &[])
				.close("w:r")
				.close("w:p")
				.close(element);
		}
		for (index, (_, blocks)) in notes
			.iter()
			.filter(|(kind, _)| *kind == endnote)
			.enumerate()
		{
			let id = (index + 1).to_string();
			xml.open(element, &[("w:id", &id)]);
			let mut mark = XmlPart::fragment();
			mark.open("w:r", &[])
				.open("w:rPr", &[])
				.empty("w:rStyle", &[("w:val", style)])
				.close("w:rPr")
				.empty(reference, &[])
				.close("w:r")
				.open("w:r", &[])
				.text_element("w:t", &[("xml:space", "preserve")], " ")
				.close("w:r");
			// the mark leads the note's first paragraph, one of its own when
			// the note opens otherwise
			if !matches!(blocks.first(), Some(BlockWrite::Paragraph(_))) {
				xml.open("w:p", &[]).raw(mark.as_str()).close("w:p");
			} else {
				self.lead = Some(mark.as_str().to_string());
			}
			self.blocks(path, blocks, &mut xml);
			self.lead = None;
			xml.close(element);
		}
		xml.close(root);
		self.package.insert(path, content_type, xml.into_bytes());
		self.package
			.relate(Some(WordRenderer::DOCUMENT), kind, path);
	}

	/// Writes `blocks` of the part at `part`.
	fn blocks(&mut self, part: &str, blocks: &[BlockWrite], xml: &mut XmlPart) {
		for block in blocks {
			match block {
				BlockWrite::Paragraph(paragraph) => {
					self.paragraph(part, paragraph, xml)
				}
				BlockWrite::Table(table) => self.table(part, table, xml),
				BlockWrite::Rule(entity) => {
					let color =
						self.resolved.computed(*entity).visual.foreground;
					xml.open("w:p", &[]).open("w:pPr", &[]).open("w:pBdr", &[]);
					let hex = color
						.map(OfficeColor::hex)
						.unwrap_or_else(|| "auto".into());
					xml.empty("w:bottom", &[
						("w:val", "single"),
						("w:sz", "6"),
						("w:space", "1"),
						("w:color", &hex),
					]);
					xml.close("w:pBdr").close("w:pPr").close("w:p");
				}
				BlockWrite::PageBreak => {
					xml.open("w:p", &[])
						.open("w:r", &[])
						.empty("w:br", &[("w:type", "page")])
						.close("w:r")
						.close("w:p");
				}
				BlockWrite::BreakAfter(entity) => {
					if self.resolved.breaks.contains(entity) {
						xml.open("w:p", &[])
							.open("w:r", &[])
							.empty("w:br", &[("w:type", "page")])
							.close("w:r")
							.close("w:p");
					}
				}
				BlockWrite::TextBox(blocks) => self.text_box(part, blocks, xml),
			}
		}
	}

	/// The style id a paragraph writes with, none for `Normal`.
	fn style_id(style: StyleName) -> Option<String> {
		match style {
			StyleName::Normal => None,
			StyleName::Heading(level) => Some(format!("Heading{level}")),
			StyleName::Quote => Some("Quote".into()),
			StyleName::Preformatted => Some("HTMLPreformatted".into()),
			StyleName::Caption => Some("Caption".into()),
			StyleName::ListItem => Some("ListParagraph".into()),
			StyleName::FootnoteText => Some("FootnoteText".into()),
			StyleName::EndnoteText => Some("EndnoteText".into()),
		}
	}

	/// Writes one paragraph: its style, numbering and what its look says
	/// beyond its chain's, a bookmark of its `id` around its runs.
	fn paragraph(
		&mut self,
		part: &str,
		paragraph: &ParagraphWrite,
		xml: &mut XmlPart,
	) {
		let look = self.resolved.computed(paragraph.entity);
		let base = self
			.resolved
			.probe(&paragraph.chain)
			.last()
			.cloned()
			.unwrap_or_default();
		xml.open("w:p", &[]);
		let mut properties = XmlPart::fragment();
		if let Some(style) = Self::style_id(paragraph.style) {
			properties.empty("w:pStyle", &[("w:val", &style)]);
		}
		if let Some((num, level)) = paragraph.numbering {
			properties
				.open("w:numPr", &[])
				.empty("w:ilvl", &[("w:val", &level.to_string())])
				.empty("w:numId", &[("w:val", &num.to_string())])
				.close("w:numPr");
		}
		if !paragraph.cell
			&& look.visual.background != base.visual.background
			&& let Some(background) = look.visual.background
		{
			Self::shading(self.styles(), &mut properties, background);
		}
		let spacing = |style: &ComputedStyle| {
			(
				self.resolved.px(style.box_style.margin.top),
				self.resolved.px(style.box_style.margin.bottom),
				self.resolved.px(style.box_style.margin.left),
			)
		};
		let (own, of_base) = (spacing(&look), spacing(&base));
		if (own.0 - of_base.0).abs() > 0.5 || (own.1 - of_base.1).abs() > 0.5 {
			properties.empty("w:spacing", &[
				("w:before", &OfficeUnits::twips(own.0).max(0).to_string()),
				("w:after", &OfficeUnits::twips(own.1).max(0).to_string()),
			]);
		}
		if (own.2 - of_base.2).abs() > 0.5 && paragraph.numbering.is_none() {
			properties.empty("w:ind", &[(
				"w:left",
				&OfficeUnits::twips(own.2).to_string(),
			)]);
		}
		if look.visual.text_align != base.visual.text_align {
			let align = match look.visual.text_align {
				TextAlign::Left => "left",
				TextAlign::Center => "center",
				TextAlign::Right => "right",
			};
			properties.empty("w:jc", &[("w:val", align)]);
		}
		if !properties.as_str().is_empty() {
			xml.open("w:pPr", &[])
				.raw(properties.as_str())
				.close("w:pPr");
		}
		if let Some(lead) = self.lead.take() {
			xml.raw(&lead);
		}
		let mark = paragraph.id.as_ref().map(|id| {
			let mark = self.next_mark();
			xml.empty("w:bookmarkStart", &[("w:id", &mark), ("w:name", id)]);
			mark
		});
		self.inline(part, &paragraph.chain, &paragraph.inline, None, xml);
		if let Some(mark) = mark {
			xml.empty("w:bookmarkEnd", &[("w:id", &mark)]);
		}
		xml.close("w:p");
	}

	fn next_mark(&mut self) -> String {
		self.marks += 1;
		(self.marks - 1).to_string()
	}

	/// Writes inline content of a paragraph whose chain is `chain`, each run
	/// inside `revision` when one is open.
	fn inline(
		&mut self,
		part: &str,
		chain: &[ProbeTag],
		inline: &[InlineWrite],
		revision: Option<(bool, Option<&str>)>,
		xml: &mut XmlPart,
	) {
		for item in inline {
			match item {
				InlineWrite::Text { text, run, look } => {
					let properties = self.run_properties(chain, run, *look);
					self.run(revision, xml, |xml, styles| {
						properties.write(styles, xml);
						let name =
							if run.deleted { "w:delText" } else { "w:t" };
						xml.text_element(
							name,
							&[("xml:space", "preserve")],
							text,
						);
					});
				}
				InlineWrite::Break { run, look } => {
					let properties = self.run_properties(chain, run, *look);
					self.run(revision, xml, |xml, styles| {
						properties.write(styles, xml);
						xml.empty("w:br", &[]);
					});
				}
				InlineWrite::Image { entity } => {
					let drawing = self.image(part, *entity);
					self.run(revision, xml, |xml, _| {
						xml.raw(&drawing);
					});
				}
				InlineWrite::Checkbox { checked } => {
					let glyph = if *checked { "\u{2612}" } else { "\u{2610}" };
					let font = "MS Gothic";
					xml.open("w:sdt", &[])
						.open("w:sdtPr", &[])
						.open("w14:checkbox", &[])
						.empty("w14:checked", &[(
							"w14:val",
							if *checked { "1" } else { "0" },
						)])
						.empty("w14:checkedState", &[
							("w14:val", "2612"),
							("w14:font", font),
						])
						.empty("w14:uncheckedState", &[
							("w14:val", "2610"),
							("w14:font", font),
						])
						.close("w14:checkbox")
						.close("w:sdtPr")
						.open("w:sdtContent", &[])
						.open("w:r", &[])
						.open("w:rPr", &[])
						.empty("w:rFonts", &[
							("w:ascii", font),
							("w:eastAsia", font),
							("w:hAnsi", font),
						])
						.close("w:rPr")
						.text_element("w:t", &[], glyph)
						.close("w:r")
						.close("w:sdtContent")
						.close("w:sdt");
				}
				InlineWrite::NoteRef { endnote, number } => {
					let (style, name) = match endnote {
						false => ("FootnoteReference", "w:footnoteReference"),
						true => ("EndnoteReference", "w:endnoteReference"),
					};
					xml.open("w:r", &[])
						.open("w:rPr", &[])
						.empty("w:rStyle", &[("w:val", style)])
						.close("w:rPr")
						.empty(name, &[("w:id", &number.to_string())])
						.close("w:r");
				}
				InlineWrite::Link { href, children } => {
					match href.strip_prefix('#') {
						Some(anchor) => xml.open("w:hyperlink", &[
							("w:anchor", anchor),
							("w:history", "1"),
						]),
						None => {
							let id = self.package.relate_external(
								Some(part),
								OpcRelationship::HYPERLINK,
								href,
							);
							xml.open("w:hyperlink", &[
								("r:id", &id),
								("w:history", "1"),
							])
						}
					};
					self.inline(part, chain, children, revision, xml);
					xml.close("w:hyperlink");
				}
				InlineWrite::Revision {
					inserted,
					datetime,
					children,
				} => {
					self.inline(
						part,
						chain,
						children,
						Some((*inserted, datetime.as_deref())),
						xml,
					);
				}
			}
		}
	}

	/// Writes one run, inside its own `w:ins` or `w:del` when `revision` is
	/// open, a reader merging equal neighbours.
	fn run(
		&mut self,
		revision: Option<(bool, Option<&str>)>,
		xml: &mut XmlPart,
		content: impl FnOnce(&mut XmlPart, &OfficeStyleSheet),
	) {
		let wrapper = revision.map(|(inserted, datetime)| {
			let name = if inserted { "w:ins" } else { "w:del" };
			let id = self.next_mark();
			let mut attributes =
				vec![("w:id", id.clone()), ("w:author", self.author.clone())];
			if let Some(datetime) = datetime {
				attributes.push(("w:date", datetime.to_string()));
			}
			let attributes = attributes
				.iter()
				.map(|(key, value)| (*key, value.as_str()))
				.collect::<Vec<_>>();
			xml.open(name, &attributes);
			name
		});
		xml.open("w:r", &[]);
		content(xml, self.styles());
		xml.close("w:r");
		if let Some(name) = wrapper {
			xml.close(name);
		}
	}

	/// What a run writes: its structure's styles and what its look says
	/// beyond what a reader reading it back resolves.
	fn run_properties(
		&self,
		chain: &[ProbeTag],
		run: &RunContext,
		look: Entity,
	) -> RunProperties {
		let own = self.resolved.run_look(look, run);
		let base = self.resolved.run_base(chain, run);
		let mut properties = RunProperties::diff(&own, &base);
		properties.style = match (run.code, run.link) {
			(true, _) => Some("HTMLCode"),
			(false, true) => Some("Hyperlink"),
			_ => None,
		};
		properties.vertical = run.vertical;
		if run.mark {
			// a mark's own background as the nearest highlight, else yellow
			properties.highlight = Some(match properties.shading.take() {
				Some(background) => WordHighlight::nearest(background),
				None => WordHighlight::DEFAULT,
			});
		}
		properties
	}

	/// The drawing of an `img`: an image part for a `data:` picture, a
	/// linked picture for any other.
	fn image(&mut self, part: &str, entity: Entity) -> String {
		let (src, alt, attributes) = self
			.resolved
			.images
			.get(&entity)
			.cloned()
			.unwrap_or_default();
		let style = self.resolved.computed(entity);
		let resolved = Vec2::new(
			style
				.box_style
				.width
				.map(|width| self.resolved.px(width))
				.unwrap_or(0.),
			style
				.box_style
				.height
				.map(|height| self.resolved.px(height))
				.unwrap_or(0.),
		);
		let picture = OfficePicture::write(
			&src,
			&alt,
			attributes,
			resolved,
			WordRenderer::CONTENT,
		);
		let blip = match &picture.source {
			PictureSource::Embedded(url) => {
				self.images += 1;
				let path = format!(
					"word/media/image{}.{}",
					self.images,
					OfficePicture::extension(url.essence())
				);
				self.package.insert(
					path.as_str(),
					url.essence(),
					url.bytes.clone(),
				);
				let id = self.package.relate(
					Some(part),
					OpcRelationship::IMAGE,
					&path,
				);
				("r:embed", id)
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
		let (cx, cy) = picture.emu();
		let (cx, cy) = (cx.to_string(), cy.to_string());
		let id = self.drawings.to_string();
		self.drawings += 1;
		let name = format!("Picture {id}");
		let mut xml = XmlPart::fragment();
		xml.open("w:drawing", &[])
			.open("wp:inline", &[
				("distT", "0"),
				("distB", "0"),
				("distL", "0"),
				("distR", "0"),
			])
			.empty("wp:extent", &[("cx", &cx), ("cy", &cy)])
			.empty("wp:docPr", &[
				("id", &id),
				("name", &name),
				("descr", &picture.alt),
			])
			.open("wp:cNvGraphicFramePr", &[])
			.empty("a:graphicFrameLocks", &[("noChangeAspect", "1")])
			.close("wp:cNvGraphicFramePr")
			.open("a:graphic", &[])
			.open("a:graphicData", &[(
				"uri",
				"http://schemas.openxmlformats.org/drawingml/2006/picture",
			)])
			.open("pic:pic", &[])
			.open("pic:nvPicPr", &[])
			.empty("pic:cNvPr", &[
				("id", "0"),
				("name", &name),
				("descr", &picture.alt),
			])
			.empty("pic:cNvPicPr", &[])
			.close("pic:nvPicPr")
			.open("pic:blipFill", &[])
			.empty("a:blip", &[(blip.0, blip.1.as_str())])
			.open("a:stretch", &[])
			.empty("a:fillRect", &[])
			.close("a:stretch")
			.close("pic:blipFill")
			.open("pic:spPr", &[])
			.open("a:xfrm", &[])
			.empty("a:off", &[("x", "0"), ("y", "0")])
			.empty("a:ext", &[("cx", &cx), ("cy", &cy)])
			.close("a:xfrm")
			.open("a:prstGeom", &[("prst", "rect")])
			.empty("a:avLst", &[])
			.close("a:prstGeom")
			.close("pic:spPr")
			.close("pic:pic")
			.close("a:graphicData")
			.close("a:graphic")
			.close("wp:inline")
			.close("w:drawing");
		xml.as_str().to_string()
	}

	/// An `aside` as a paragraph holding an inline text box of its blocks,
	/// as wide as the page's content box and tall enough for its words.
	fn text_box(
		&mut self,
		part: &str,
		blocks: &[BlockWrite],
		xml: &mut XmlPart,
	) {
		let mut content = XmlPart::fragment();
		self.blocks(part, blocks, &mut content);
		if blocks.is_empty() {
			content.empty("w:p", &[]);
		}
		// a line of about 90 characters per paragraph's worth of words
		let lines = blocks
			.iter()
			.map(|block| match block {
				BlockWrite::Paragraph(paragraph) => {
					1 + InlineWrite::text_length(&paragraph.inline) / 90
				}
				_ => 1,
			})
			.sum::<usize>()
			.max(1);
		let (cx, cy) = (
			OfficeUnits::emu(WordRenderer::CONTENT.x).to_string(),
			OfficeUnits::emu(lines as f32 * 24. + 16.).to_string(),
		);
		let id = self.drawings.to_string();
		self.drawings += 1;
		let name = format!("Text Box {id}");
		xml.open("w:p", &[])
			.open("w:r", &[])
			.open("mc:AlternateContent", &[])
			.open("mc:Choice", &[("Requires", "wps")])
			.open("w:drawing", &[])
			.open("wp:inline", &[
				("distT", "0"),
				("distB", "0"),
				("distL", "0"),
				("distR", "0"),
			])
			.empty("wp:extent", &[("cx", &cx), ("cy", &cy)])
			.empty("wp:docPr", &[("id", &id), ("name", &name)])
			.empty("wp:cNvGraphicFramePr", &[])
			.open("a:graphic", &[])
			.open("a:graphicData", &[(
				"uri",
				"http://schemas.microsoft.com/office/word/2010/wordprocessingShape",
			)])
			.open("wps:wsp", &[])
			.empty("wps:cNvSpPr", &[("txBox", "1")])
			.open("wps:spPr", &[])
			.open("a:xfrm", &[])
			.empty("a:off", &[("x", "0"), ("y", "0")])
			.empty("a:ext", &[("cx", &cx), ("cy", &cy)])
			.close("a:xfrm")
			.open("a:prstGeom", &[("prst", "rect")])
			.empty("a:avLst", &[])
			.close("a:prstGeom")
			.empty("a:noFill", &[])
			.open("a:ln", &[("w", "6350")])
			.open("a:solidFill", &[])
			.empty("a:schemeClr", &[("val", "tx1")])
			.close("a:solidFill")
			.close("a:ln")
			.close("wps:spPr")
			.open("wps:txbx", &[])
			.open("w:txbxContent", &[])
			.raw(content.as_str())
			.close("w:txbxContent")
			.close("wps:txbx")
			.open("wps:bodyPr", &[
				("rot", "0"),
				("vert", "horz"),
				("wrap", "square"),
				("lIns", "91440"),
				("tIns", "45720"),
				("rIns", "91440"),
				("bIns", "45720"),
				("anchor", "t"),
			])
			.empty("a:spAutoFit", &[])
			.close("wps:bodyPr")
			.close("wps:wsp")
			.close("a:graphicData")
			.close("a:graphic")
			.close("wp:inline")
			.close("w:drawing")
			.close("mc:Choice")
			.close("mc:AlternateContent")
			.close("w:r")
			.close("w:p");
	}

	/// Writes a table across the page's width with single borders, its
	/// caption a paragraph above it and its `id` a bookmark before it.
	fn table(&mut self, part: &str, table: &TableWrite, xml: &mut XmlPart) {
		if let Some(caption) = &table.caption {
			self.paragraph(part, caption, xml);
		}
		let mark = table.id.as_ref().map(|id| {
			let mark = self.next_mark();
			xml.empty("w:bookmarkStart", &[("w:id", &mark), ("w:name", id)]);
			mark
		});
		let width = WordRenderer::CONTENT_TWIPS / table.columns as i64;
		xml.open("w:tbl", &[])
			.open("w:tblPr", &[])
			.empty("w:tblW", &[("w:w", "5000"), ("w:type", "pct")])
			.open("w:tblBorders", &[]);
		for side in ["top", "left", "bottom", "right", "insideH", "insideV"] {
			xml.empty(&format!("w:{side}"), &[
				("w:val", "single"),
				("w:sz", "4"),
				("w:space", "0"),
				("w:color", "auto"),
			]);
		}
		xml.close("w:tblBorders")
			.empty("w:tblLook", &[
				("w:val", "04A0"),
				("w:firstRow", "1"),
				("w:lastRow", "0"),
				("w:firstColumn", "1"),
				("w:lastColumn", "0"),
				("w:noHBand", "0"),
				("w:noVBand", "1"),
			])
			.close("w:tblPr")
			.open("w:tblGrid", &[]);
		for _ in 0..table.columns {
			xml.empty("w:gridCol", &[("w:w", &width.to_string())]);
		}
		xml.close("w:tblGrid");
		for row in &table.rows {
			xml.open("w:tr", &[]);
			if row.header {
				xml.open("w:trPr", &[])
					.empty("w:tblHeader", &[])
					.close("w:trPr");
			}
			for cell in &row.cells {
				self.cell(part, cell, width, xml);
			}
			xml.close("w:tr");
		}
		xml.close("w:tbl");
		if let Some(mark) = mark {
			xml.empty("w:bookmarkEnd", &[("w:id", &mark)]);
		}
	}

	/// Writes one cell, its merges, its shading, and its blocks ending in a
	/// paragraph as a cell must.
	fn cell(
		&mut self,
		part: &str,
		cell: &CellWrite,
		width: i64,
		xml: &mut XmlPart,
	) {
		xml.open("w:tc", &[]).open("w:tcPr", &[]);
		let cell_width = (width * cell.span.x as i64).to_string();
		xml.empty("w:tcW", &[("w:w", &cell_width), ("w:type", "dxa")]);
		if cell.span.x > 1 {
			xml.empty("w:gridSpan", &[("w:val", &cell.span.x.to_string())]);
		}
		match (cell.entity, cell.restart) {
			(None, _) => {
				xml.empty("w:vMerge", &[]);
			}
			(Some(_), true) => {
				xml.empty("w:vMerge", &[("w:val", "restart")]);
			}
			(Some(_), false) => {}
		}
		if let Some(entity) = cell.entity {
			let look = self.resolved.computed(entity).visual.background;
			let base = self
				.resolved
				.probe(&cell.chain)
				.last()
				.and_then(|style| style.visual.background);
			if look != base
				&& let Some(background) = look
			{
				Self::shading(self.styles(), xml, background);
			}
		}
		xml.close("w:tcPr");
		self.blocks(part, &cell.blocks, xml);
		let ends_in_paragraph =
			cell.blocks
				.iter()
				.rev()
				.find(|block| !matches!(block, BlockWrite::BreakAfter(_)))
				.is_some_and(|block| {
					matches!(
						block,
						BlockWrite::Paragraph(_)
							| BlockWrite::Rule(_) | BlockWrite::PageBreak
							| BlockWrite::TextBox(_)
					)
				});
		if !ends_in_paragraph {
			xml.empty("w:p", &[]);
		}
		xml.close("w:tc");
	}

	/// The styles part: the document's defaults from the root's look, then
	/// each named style with its tag's look, a heading with Word's own bold
	/// and size where nothing resolves one.
	fn styles_xml(&self) -> Vec<u8> {
		let styles = self.styles();
		let defaults = styles.defaults();
		let chain = |tags: &[&str]| {
			tags.iter()
				.map(|tag| ProbeTag::new(*tag))
				.collect::<Chain>()
		};
		let mut xml = XmlPart::new();
		xml.open("w:styles", &WordRenderer::NAMESPACES)
			.open("w:docDefaults", &[])
			.open("w:rPrDefault", &[])
			.open("w:rPr", &[]);
		let font = WordRenderer::FONT;
		xml.empty("w:rFonts", &[
			("w:ascii", font),
			("w:eastAsia", font),
			("w:hAnsi", font),
			("w:cs", font),
		]);
		if let Some(color) = defaults.color {
			Self::color(
				styles,
				&mut xml,
				"w:color",
				"w:val",
				"w:themeColor",
				color,
			);
		}
		let half = OfficeUnits::half_points(
			defaults.font_size_px.unwrap_or(OfficeUnits::PX_PER_REM),
		)
		.to_string();
		xml.empty("w:sz", &[("w:val", &half)])
			.empty("w:szCs", &[("w:val", &half)]);
		xml.close("w:rPr")
			.close("w:rPrDefault")
			.open("w:pPrDefault", &[])
			.open("w:pPr", &[])
			.empty("w:spacing", &[
				("w:after", "0"),
				("w:line", "259"),
				("w:lineRule", "auto"),
			])
			.close("w:pPr")
			.close("w:pPrDefault")
			.close("w:docDefaults");
		let normal = chain(&["p"]);
		self.paragraph_style(
			&mut xml,
			"Normal",
			"Normal",
			None,
			&normal,
			None,
			|_| {},
		);
		for level in 1..=6u8 {
			let probe = chain(&[&format!("h{level}")]);
			// Word's own heading look where no rule gives one
			let format_size =
				[20., 16., 14., 12., 12., 12.][level as usize - 1];
			self.paragraph_style(
				&mut xml,
				&format!("Heading{level}"),
				&format!("heading {level}"),
				Some(&level),
				&probe,
				Some(format_size),
				|_| {},
			);
		}
		self.paragraph_style(
			&mut xml,
			"Quote",
			"Quote",
			None,
			&chain(&["blockquote", "p"]),
			None,
			|_| {},
		);
		self.paragraph_style(
			&mut xml,
			"HTMLPreformatted",
			"HTML Preformatted",
			None,
			&chain(&["pre"]),
			None,
			|rpr| rpr.monospace = true,
		);
		self.paragraph_style(
			&mut xml,
			"Caption",
			"caption",
			None,
			&chain(&["table", "caption"]),
			None,
			|_| {},
		);
		self.paragraph_style(
			&mut xml,
			"ListParagraph",
			"List Paragraph",
			None,
			&chain(&["ul", "li"]),
			None,
			|_| {},
		);
		let note = vec![
			ProbeTag::with_class("div", "footnote-definition"),
			ProbeTag::new("p"),
		];
		self.paragraph_style(
			&mut xml,
			"FootnoteText",
			"footnote text",
			None,
			&note,
			None,
			|_| {},
		);
		self.paragraph_style(
			&mut xml,
			"EndnoteText",
			"endnote text",
			None,
			&note,
			None,
			|_| {},
		);
		// character styles: each tag's look over a paragraph's
		let paragraph = self.resolved.probe_look(&normal);
		for (id, name, tags, monospace) in [
			("HTMLCode", "HTML Code", ["p", "code"], true),
			("Hyperlink", "Hyperlink", ["p", "a"], false),
		] {
			let mut properties = RunProperties::diff(
				&self.resolved.probe_look(&chain(&tags)),
				&paragraph,
			);
			properties.monospace = monospace;
			Self::character_style(styles, &mut xml, id, name, &properties);
		}
		for (id, name) in [
			("FootnoteReference", "footnote reference"),
			("EndnoteReference", "endnote reference"),
		] {
			let properties = RunProperties {
				vertical: Some("superscript"),
				..default()
			};
			Self::character_style(styles, &mut xml, id, name, &properties);
		}
		xml.close("w:styles");
		xml.into_bytes()
	}

	/// Writes the paragraph style `id`: its spacing, alignment and shading
	/// from its chain's look and its run properties over the defaults.
	#[allow(clippy::too_many_arguments)]
	fn paragraph_style(
		&self,
		xml: &mut XmlPart,
		id: &str,
		name: &str,
		outline: Option<&u8>,
		chain: &[ProbeTag],
		heading_size: Option<f32>,
		adjust: impl FnOnce(&mut RunProperties),
	) {
		let styles = self.styles();
		let probe = self
			.resolved
			.probe(chain)
			.last()
			.cloned()
			.unwrap_or_default();
		let look = OfficeLook::from_computed(&probe);
		let mut properties = RunProperties::diff(&look, styles.defaults());
		if let Some(size) = heading_size {
			properties.bold = properties.bold.or(Some(true));
			properties.size_px =
				properties.size_px.or(Some(OfficeUnits::px_from_pt(size)));
		}
		adjust(&mut properties);
		let mut attributes = vec![("w:type", "paragraph"), ("w:styleId", id)];
		if id == "Normal" {
			attributes.push(("w:default", "1"));
		}
		xml.open("w:style", &attributes)
			.empty("w:name", &[("w:val", name)]);
		if id != "Normal" {
			xml.empty("w:basedOn", &[("w:val", "Normal")])
				.empty("w:next", &[("w:val", "Normal")]);
		}
		xml.empty("w:qFormat", &[]).open("w:pPr", &[]);
		if outline.is_some() {
			xml.empty("w:keepNext", &[]);
		}
		if let Some(background) = probe.visual.background {
			Self::shading(styles, xml, background);
		}
		let (before, after, left) = (
			self.resolved.px(probe.box_style.margin.top),
			self.resolved.px(probe.box_style.margin.bottom),
			self.resolved.px(probe.box_style.margin.left),
		);
		xml.empty("w:spacing", &[
			("w:before", &OfficeUnits::twips(before).max(0).to_string()),
			("w:after", &OfficeUnits::twips(after).max(0).to_string()),
		]);
		if left > 0.5 {
			xml.empty("w:ind", &[(
				"w:left",
				&OfficeUnits::twips(left).to_string(),
			)]);
		}
		match probe.visual.text_align {
			TextAlign::Center => {
				xml.empty("w:jc", &[("w:val", "center")]);
			}
			TextAlign::Right => {
				xml.empty("w:jc", &[("w:val", "right")]);
			}
			TextAlign::Left => {}
		}
		if let Some(level) = outline {
			xml.empty("w:outlineLvl", &[("w:val", &(level - 1).to_string())]);
		}
		xml.close("w:pPr");
		if properties != RunProperties::default() {
			xml.open("w:rPr", &[]);
			properties.write_inner(styles, xml);
			xml.close("w:rPr");
		}
		xml.close("w:style");
	}

	/// Writes the character style `id` of `properties`.
	fn character_style(
		styles: &OfficeStyleSheet,
		xml: &mut XmlPart,
		id: &str,
		name: &str,
		properties: &RunProperties,
	) {
		xml.open("w:style", &[("w:type", "character"), ("w:styleId", id)])
			.empty("w:name", &[("w:val", name)])
			.empty("w:uiPriority", &[("w:val", "99")])
			.empty("w:unhideWhenUsed", &[]);
		if *properties != RunProperties::default() {
			xml.open("w:rPr", &[]);
			properties.write_inner(styles, xml);
			xml.close("w:rPr");
		}
		xml.close("w:style");
	}

	/// The numbering part: one bullet and one decimal definition, and each
	/// list its own `w:num` restarting its count.
	fn numbering_xml(nums: &[bool]) -> Vec<u8> {
		let mut xml = XmlPart::new();
		xml.open("w:numbering", &WordRenderer::NAMESPACES);
		for (id, ordered) in [("0", false), ("1", true)] {
			xml.open("w:abstractNum", &[("w:abstractNumId", id)])
				.empty("w:multiLevelType", &[("w:val", "multilevel")]);
			for level in 0..9u32 {
				let (format, text) = match ordered {
					true => ("decimal", format!("%{}.", level + 1)),
					false => (
						"bullet",
						["\u{2022}", "\u{25E6}", "\u{25AA}"]
							[level as usize % 3]
							.to_string(),
					),
				};
				let indent = (720 * (level + 1)).to_string();
				xml.open("w:lvl", &[("w:ilvl", &level.to_string())])
					.empty("w:start", &[("w:val", "1")])
					.empty("w:numFmt", &[("w:val", format)])
					.empty("w:lvlText", &[("w:val", &text)])
					.empty("w:lvlJc", &[("w:val", "left")])
					.open("w:pPr", &[])
					.empty("w:ind", &[
						("w:left", &indent),
						("w:hanging", "360"),
					])
					.close("w:pPr")
					.close("w:lvl");
			}
			xml.close("w:abstractNum");
		}
		for (index, ordered) in nums.iter().enumerate() {
			xml.open("w:num", &[("w:numId", &(index + 1).to_string())])
				.empty("w:abstractNumId", &[(
					"w:val",
					if *ordered { "1" } else { "0" },
				)]);
			for level in 0..9 {
				xml.open("w:lvlOverride", &[("w:ilvl", &level.to_string())])
					.empty("w:startOverride", &[("w:val", "1")])
					.close("w:lvlOverride");
			}
			xml.close("w:num");
		}
		xml.close("w:numbering");
		xml.into_bytes()
	}

	/// The settings part: the page background shown, the notes' separators
	/// and Word 2013's compatibility.
	fn settings_xml(&self, notes: [bool; 2]) -> Vec<u8> {
		let mut xml = XmlPart::new();
		xml.open("w:settings", &WordRenderer::NAMESPACES);
		if self.styles().defaults().background.is_some() {
			xml.empty("w:displayBackgroundShape", &[]);
		}
		xml.empty("w:defaultTabStop", &[("w:val", "720")]);
		for (written, name) in notes.into_iter().zip(["footnote", "endnote"]) {
			if written {
				xml.open(&format!("w:{name}Pr"), &[])
					.empty(&format!("w:{name}"), &[("w:id", "-1")])
					.empty(&format!("w:{name}"), &[("w:id", "0")])
					.close(&format!("w:{name}Pr"));
			}
		}
		xml.open("w:compat", &[])
			.empty("w:compatSetting", &[
				("w:name", "compatibilityMode"),
				("w:uri", "http://schemas.microsoft.com/office/word"),
				("w:val", "15"),
			])
			.close("w:compat")
			.close("w:settings");
		xml.into_bytes()
	}
}

impl InlineWrite {
	/// The characters of text `inline` holds.
	fn text_length(inline: &[InlineWrite]) -> usize {
		inline
			.iter()
			.map(|item| match item {
				InlineWrite::Text { text, .. } => text.chars().count(),
				InlineWrite::Link { children, .. }
				| InlineWrite::Revision { children, .. } => Self::text_length(children),
				_ => 1,
			})
			.sum()
	}
}

impl Installer for WordRenderer {
	fn install(self, app: &mut App) { DocRenderers::register_on(app, self); }
}

impl DocRenderer for WordRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Docx] }

	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[MediaType::Docx])?;
		let (root, request) = (cx.entity, cx.request);
		let bytes = Self::write(cx.world, root, request)?;
		MediaBytes::new(MediaType::Docx, bytes).xok()
	}
}

#[cfg(test)]
mod test {
	use super::super::office_test::*;
	use super::super::word_parser::test::*;
	use crate::prelude::*;
	use crate::style::material::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// `root` written as a Word file for `request`.
	fn write(
		world: &mut World,
		root: Entity,
		request: RequestParts,
	) -> MediaBytes {
		super::super::office_test::write(world, root, MediaType::Docx, request)
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
	/// answering each read's HTML.
	fn round_trip(
		bytes: &MediaBytes,
		themed: bool,
		options: OfficeReadOptions,
	) -> (String, String, MediaBytes) {
		let read = |bytes: &MediaBytes| {
			let mut world = formats(themed);
			world
				.resource_mut::<DocParsers>()
				.register(WordParser::default().with_options(options));
			parse_in(world, bytes).unwrap()
		};
		let (mut first, root) = read(bytes);
		let written = write(&mut first, root, default());
		let (mut again, again_root) = read(&written);
		// the properties too
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

	/// The model's round trip: the covering file read, written and read
	/// again is the same tree, with the Material theme and without, where
	/// a theme slot reads verbatim since no token would resolve.
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
		// headings named by their built-in names and outline levels
		part(&written, "word/styles.xml")
			.xpect_contains("<w:style w:type=\"paragraph\" w:styleId=\"Heading1\"><w:name w:val=\"heading 1\"/>")
			.xpect_contains("<w:outlineLvl w:val=\"0\"/>");
		part(&written, "word/document.xml")
			.xpect_contains("<w:pgSz w:w=\"11906\" w:h=\"16838\"/>")
			.xpect_contains("<w:headerReference w:type=\"default\"");
	}

	/// A table nested in a cell, and a cell of several blocks, round trip,
	/// the paragraph Word requires closing the cell dropped on the read.
	#[beet_core::test]
	fn round_trips_nested_tables() {
		let cell = |content: &str| format!("<w:tc>{content}</w:tc>");
		let inner = format!(
			"<w:tbl><w:tr>{}</w:tr></w:tbl>",
			cell(&paragraph("", &run("", "inner")))
		);
		let bytes = WordFixture::new(&format!(
			"<w:tbl><w:tr>{}{}</w:tr></w:tbl>",
			cell(&format!(
				"{}{inner}<w:p/>",
				paragraph("", &run("", "outer"))
			)),
			cell(
				&[
					paragraph("", &run("", "one")),
					paragraph("", &run("", "two"))
				]
				.concat()
			),
		))
		.bytes();
		let (first, again, _) = round_trip(&bytes, false, default());
		first.clone().xpect_contains(
			"<td><p>outer</p><table><tbody><tr><td>inner</td></tr></tbody></table></td><td><p>one</p><p>two</p></td>",
		);
		again.xpect_eq(first);
	}

	/// A themed page written for each scheme writes that scheme's tones as
	/// its default text colour and page background, its theme part's slots
	/// that scheme's palette.
	#[beet_core::test]
	fn writes_each_schemes_tones() {
		let mut world = formats(true);
		let root = world
			.spawn((rsx! { <p>"words"</p> }, Classes::new([classes::PAGE])))
			.id();
		let mut backgrounds = Vec::new();
		for scheme in ["dark", "light"] {
			let written = write(
				&mut world,
				root,
				RequestParts::default().with_param("color-scheme", scheme),
			);
			let media = MediaContext::new(Medium::Print)
				.with_scheme(ColorScheme::parse(scheme).unwrap());
			let (surface, on_surface) =
				world.with_state::<RuleSetQuery, _>(|query| {
					let memo = &mut CascadeMemo::for_media(media);
					(
						OfficeColor::hex(
							query.resolve(root, colors::Surface, memo).unwrap(),
						),
						OfficeColor::hex(
							query
								.resolve(root, colors::OnSurface, memo)
								.unwrap(),
						),
					)
				});
			part(&written, "word/document.xml").xpect_contains(format!(
				"<w:background w:color=\"{surface}\" w:themeColor=\"background1\"/>"
			));
			part(&written, "word/styles.xml").xpect_contains(format!(
				"<w:color w:val=\"{on_surface}\" w:themeColor=\"text1\"/>"
			));
			part(&written, "word/settings.xml")
				.xpect_contains("<w:displayBackgroundShape/>");
			OfficeTheme::parse(
				&OpcPackage::read_xml(
					part(&written, "word/theme/theme1.xml").as_bytes(),
				)
				.unwrap(),
			)
			.color(ThemeSlot::Light1)
			.xmap(OfficeColor::hex)
			.xpect_eq(surface.clone());
			backgrounds.push(surface);
		}
		(backgrounds[0] != backgrounds[1]).xpect_true();
	}

	/// A markdown page written as a Word file reads back holding its
	/// structure, and that read round trips.
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn writes_a_markdown_page() {
		let image = DataUrl::new(
			"image/png",
			super::super::office_picture::test::png(),
		)
		.to_string();
		let markdown = format!(
			"# Title\n\nSome *emphasis*, **strong** and `code`, [a link](https://example.com), a note[^1].\n\n\
			 ## Lists\n\n- one\n  - nested\n- two\n\n1. first\n2. second\n\n\
			 | a | b |\n|---|---|\n| x | y |\n\n\
			 <table><thead><tr><th>h</th><th>i</th></tr></thead><tbody><tr><td colspan=\"2\">wide</td></tr></tbody></table>\n\n\
			 ```rust\nfn main() {{}}\n```\n\n![a picture]({image})\n\n---\n\n[^1]: The note.\n"
		);
		let (mut world, root) = parse_in(
			formats(false),
			&MediaBytes::new(MediaType::Markdown, markdown.into_bytes()),
		)
		.unwrap();
		let written = write(&mut world, root, default());
		let read =
			|bytes: &MediaBytes| parse_in(formats(false), bytes).unwrap();
		let (mut again, again_root) = read(&written);
		let first = html(&mut again, again_root);
		first
			.clone()
			.xpect_contains("<h1")
			.xpect_contains(">Title</h1>")
			.xpect_contains("<em>emphasis</em>")
			.xpect_contains("<strong>strong</strong>")
			.xpect_contains("<code>code</code>")
			.xpect_contains("<a href=\"https://example.com\">a link</a>")
			.xpect_contains("<sup><a href=\"#fn-1\">1</a></sup>")
			.xpect_contains("<ul><li>one<ul><li>nested</li></ul></li><li>two</li></ul><ol><li>first</li><li>second</li></ol>")
			.xpect_contains("<thead><tr><th>a</th><th>b</th></tr></thead><tbody><tr><td>x</td><td>y</td></tr></tbody>")
			.xpect_contains("<tbody><tr><td colspan=\"2\">wide</td></tr></tbody>")
			.xpect_contains("<pre><code>fn main() {}</code></pre>")
			.xpect_contains("<img src=\"data:image/png;base64,")
			.xpect_contains("<hr />")
			.xpect_contains("<div id=\"fn-1\" class=\"footnote-definition\"><p>The note.</p></div>");
		let rewritten = write(&mut again, again_root, default());
		let (mut third, third_root) = read(&rewritten);
		html(&mut third, third_root).xpect_eq(first);
	}

	/// LibreOffice converts a written file headless to text, the proxy for
	/// opening without a repair prompt, its headings and table intact.
	/// Skips where `soffice` is absent.
	#[beet_core::test]
	fn libreoffice_reads_a_written_file() {
		if !has_soffice() {
			return;
		}
		let (mut world, root) = parse(&covering()).unwrap();
		let written = write(&mut world, root, default());
		soffice_convert(&written, "covering.docx", "txt:Text")
			.xpect_contains("Cover")
			.xpect_contains("Looks")
			.xpect_contains("Name")
			.xpect_contains("wide")
			.xpect_contains("tall");
	}
}
