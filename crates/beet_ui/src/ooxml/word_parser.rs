//! [`WordParser`], a Word file read as plain HTML, by the [module docs](super)'
//! Word mapping.
use super::office_picture::*;
use super::office_style::*;
use super::read_element::*;
use crate::prelude::*;
use beet_core::prelude::*;

/// Reads a Word file into the elements its HTML analogy names: the default
/// header, the body's blocks, the notes and the default footer, with nothing
/// Office-specific left on any entity. Named styles are matched by their
/// built-in names, never their ids, which Word localizes. See the
/// [module docs](super) for the mapping and what it loses.
#[derive(Debug, Default, Clone, Get, SetWith)]
pub struct WordParser {
	/// How the file's looks are read.
	options: OfficeReadOptions,
}

/// A highlight colour a Word run names, `w:highlight`'s value.
pub(crate) struct WordHighlight;

impl WordHighlight {
	/// The highlight a `mark` with no look of its own writes as.
	pub const DEFAULT: &str = "yellow";
	/// Every highlight colour by its name, `ST_HighlightColor`.
	pub const COLORS: [(&str, &str); 16] = [
		("black", "000000"),
		("blue", "0000FF"),
		("cyan", "00FFFF"),
		("green", "00FF00"),
		("magenta", "FF00FF"),
		("red", "FF0000"),
		("yellow", "FFFF00"),
		("white", "FFFFFF"),
		("darkBlue", "000080"),
		("darkCyan", "008080"),
		("darkGreen", "008000"),
		("darkMagenta", "800080"),
		("darkRed", "800000"),
		("darkYellow", "808000"),
		("darkGray", "808080"),
		("lightGray", "C0C0C0"),
	];

	/// The colour the highlight `name` draws.
	pub fn color(name: &str) -> Option<Color> {
		Self::COLORS
			.iter()
			.find(|(found, _)| *found == name)
			.and_then(|(_, hex)| OfficeColor::parse_hex(hex))
	}

	/// The highlight nearest `color`, by the distance of its channels.
	pub fn nearest(color: Color) -> &'static str {
		let target = color.to_srgba();
		Self::COLORS
			.iter()
			.filter_map(|(name, hex)| {
				let found = OfficeColor::parse_hex(hex)?.to_srgba();
				let distance = (found.red - target.red).powi(2)
					+ (found.green - target.green).powi(2)
					+ (found.blue - target.blue).powi(2);
				Some((*name, distance))
			})
			.min_by(|left, right| left.1.total_cmp(&right.1))
			.map_or(Self::DEFAULT, |(name, _)| name)
	}
}

/// What a named style means to the reader, matched by its built-in name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StyleKind {
	/// `Title` as 1, `heading 1` to `heading 6`.
	Heading(u8),
	/// `Quote` or `Intense Quote`.
	Quote,
	/// `HTML Preformatted`.
	Preformatted,
	/// `HTML Code`, a character style.
	Code,
	/// `caption`.
	Caption,
}

/// What the reader knows of a file's named styles.
#[derive(Debug, Default)]
struct WordStyles {
	/// Each style's kind, by its id.
	kinds: HashMap<SmolStr, StyleKind>,
	/// Each paragraph style's font size in half points, through the styles
	/// it is based on, where one says.
	sizes: HashMap<SmolStr, u32>,
	/// Each paragraph style's list numbering, its `numId` and `ilvl`.
	numbering: HashMap<SmolStr, (SmolStr, u8)>,
	/// The paragraph style a paragraph naming none takes.
	default_paragraph: Option<SmolStr>,
	/// The document's default font size in half points, `w:docDefaults`'.
	default_size: u32,
}

/// Whether each level of each list counts, by `numId` and `ilvl`.
#[derive(Debug, Default)]
struct WordNumbering {
	ordered: HashMap<(SmolStr, u8), bool>,
}

/// One block a part's body reads, before lists, quotes and code are grouped.
#[derive(Debug)]
enum BlockRead {
	/// A paragraph by the kind its style and numbering make it.
	Paragraph {
		kind: ParagraphKind,
		element: ReadElement,
	},
	/// Any other block, ie a table, an `hr` or a page break.
	Other(ReadElement),
}

/// What a paragraph reads as.
#[derive(Debug, Clone, PartialEq)]
enum ParagraphKind {
	Plain,
	Heading(u8),
	Quote,
	Preformatted,
	Caption,
	/// A list item at `level`, of the list `num`.
	Item {
		level: u8,
		ordered: bool,
		num: SmolStr,
	},
}

/// A field being read across runs: its instruction until its separator,
/// then its result.
#[derive(Debug, Default)]
struct Field {
	instruction: String,
	/// Whether its separator has passed, so its runs are its result.
	in_result: bool,
	/// The `a` its result reads inside, for a `HYPERLINK` field.
	link: Option<ReadElement>,
}

/// One piece of a paragraph's content inside the elements wrapping it,
/// outermost first, before equal wrappers are merged.
#[derive(Debug, Clone)]
struct Segment {
	wrappers: Vec<ReadElement>,
	leaf: Leaf,
}

#[derive(Debug, Clone)]
enum Leaf {
	Text(String),
	Element(ReadElement),
	/// A page break, which splits its paragraph.
	PageBreak,
}

/// The inline content of one paragraph as it is read.
#[derive(Debug, Default)]
struct InlineRead {
	segments: Vec<Segment>,
	/// The wrappers open around the content being read: links, revisions.
	context: Vec<ReadElement>,
	fields: Vec<Field>,
	/// The bookmarks opening in the paragraph.
	bookmarks: Vec<SmolStr>,
	/// The text boxes it holds, read as blocks after it.
	asides: Vec<ReadElement>,
	/// The paragraph style's font size in half points, a run's size compared
	/// with it.
	base_size: u32,
}

/// A Word package being read.
struct WordRead<'a> {
	package: &'a OpcPackage,
	styles: WordStyles,
	numbering: WordNumbering,
	theme: OfficeTheme,
	/// The footnotes referenced so far, by id in reference order.
	footnotes: Vec<SmolStr>,
	/// The endnotes referenced so far.
	endnotes: Vec<SmolStr>,
	/// Whether the next text is a note's first, its leading space dropped.
	note_start: bool,
}

impl WordParser {
	/// The prefixes of an `mc:Choice` the reader reads: a text box shape,
	/// its group and Word 2010's additions, a checkbox among them.
	const CHOICES: [&str; 4] = ["wps", "wpg", "wp14", "w14"];

	/// Reads `package`'s Word document into `root`.
	fn read(
		&self,
		world: &mut World,
		root: Entity,
		package: &OpcPackage,
	) -> Result {
		let main = package
			.main_part()
			.ok_or_else(|| bevyhow!("the Word file has no main document part"))?
			.to_string();
		let xml = |kind: &str| -> Result<Option<Vec<BsxNode>>> {
			package
				.related(Some(&main), kind)
				.map(|path| package.xml(path))
				.transpose()
				.map(Option::flatten)
		};
		let mut read = WordRead {
			package,
			styles: xml(OpcRelationship::STYLES)?
				.map(|nodes| WordStyles::parse(&nodes))
				.unwrap_or_default(),
			numbering: xml(OpcRelationship::NUMBERING)?
				.map(|nodes| WordNumbering::parse(&nodes))
				.unwrap_or_default(),
			theme: xml(OpcRelationship::THEME)?
				.map(|nodes| OfficeTheme::parse(&nodes))
				.unwrap_or_default(),
			footnotes: Vec::new(),
			endnotes: Vec::new(),
			note_start: false,
		};
		let nodes = package.xml(&main)?.unwrap_or_default();
		let body = BsxNode::document_element(&nodes)
			.and_then(|document| document.child("body"))
			.ok_or_else(|| bevyhow!("the Word document has no body"))?;
		let mut out = Vec::new();
		// the default header and footer of the body's last section
		let section = body.child("sectPr");
		let part_of = |reference: &str| {
			section
				.into_iter()
				.flat_map(|section| section.children_named(reference))
				.find(|found| {
					found
						.attribute_local("type")
						.is_none_or(|kind| kind == "default")
				})
				.and_then(|found| found.prefixed_attribute("id"))
				.and_then(|id| package.relationship(Some(&main), id))
				.map(|relationship| relationship.target.to_string())
		};
		let (header, footer) =
			(part_of("headerReference"), part_of("footerReference"));
		if let Some(path) = header {
			out.extend(read.part_element(&path, "header")?);
		}
		out.extend(read.blocks(&main, body));
		out.extend(read.notes(&main)?);
		if let Some(path) = footer {
			out.extend(read.part_element(&path, "footer")?);
		}
		for element in out {
			element.spawn(world, root, &self.options, &read.theme)?;
		}
		Ok(())
	}
}

impl WordStyles {
	/// The styles part's kinds, sizes and numbering.
	fn parse(nodes: &[BsxNode]) -> Self {
		let mut styles = Self {
			default_size: 20,
			..default()
		};
		let Some(root) = BsxNode::document_element(nodes) else {
			return styles;
		};
		if let Some(size) = root
			.child("docDefaults")
			.and_then(|defaults| defaults.child("rPrDefault"))
			.and_then(|default| default.child("rPr"))
			.and_then(Self::size)
		{
			styles.default_size = size;
		}
		// each style's own size and base, to resolve through the chain
		let mut own =
			HashMap::<SmolStr, (Option<u32>, Option<SmolStr>)>::default();
		for style in root.children_named("style") {
			let Some(id) = style.attribute_local("styleId").map(SmolStr::new)
			else {
				continue;
			};
			let name = style
				.child("name")
				.and_then(|name| name.attribute_local("val"))
				.unwrap_or_default();
			if let Some(kind) = StyleKind::from_name(name) {
				styles.kinds.insert(id.clone(), kind);
			}
			if style.attribute_local("type") != Some("paragraph") {
				continue;
			}
			if matches!(style.attribute_local("default"), Some("1" | "true")) {
				styles.default_paragraph = Some(id.clone());
			}
			if let Some((num, level)) =
				style.child("pPr").and_then(WordRead::numbering_of)
			{
				styles.numbering.insert(id.clone(), (num, level));
			}
			own.insert(
				id,
				(
					style.child("rPr").and_then(Self::size),
					style
						.child("basedOn")
						.and_then(|base| base.attribute_local("val"))
						.map(SmolStr::new),
				),
			);
		}
		for id in own.keys() {
			// a cycle of bases ends at the styles already walked
			let mut walked = Vec::<&SmolStr>::new();
			let mut at = Some(id);
			while let Some(current) =
				at.filter(|current| !walked.contains(current))
			{
				walked.push(current);
				let Some((size, base)) = own.get(current) else {
					break;
				};
				if let Some(size) = size {
					styles.sizes.insert(id.clone(), *size);
					break;
				}
				at = base.as_ref();
			}
		}
		styles
	}

	/// A run properties element's size, `w:sz` in half points.
	fn size(properties: &BsxElement) -> Option<u32> {
		properties
			.child("sz")
			.and_then(|size| size.attribute_local("val"))
			.and_then(|size| size.parse().ok())
	}

	/// The kind of the style `id`, by its name, else by the id Word gives
	/// the built-in style in English where the file defines no such style.
	fn kind(&self, id: &str) -> Option<StyleKind> {
		self.kinds
			.get(id)
			.copied()
			.or_else(|| StyleKind::from_id(id))
	}

	/// The font size of a paragraph styled `style` in half points.
	fn size_of(&self, style: Option<&str>) -> u32 {
		style
			.or(self.default_paragraph.as_deref())
			.and_then(|style| self.sizes.get(style))
			.copied()
			.unwrap_or(self.default_size)
	}
}

impl StyleKind {
	/// The kind a style's built-in name means, matched without case.
	fn from_name(name: &str) -> Option<Self> {
		let name = name.trim().to_lowercase();
		match name.as_str() {
			"title" => Some(Self::Heading(1)),
			"quote" | "intense quote" => Some(Self::Quote),
			"html preformatted" => Some(Self::Preformatted),
			"html code" => Some(Self::Code),
			"caption" => Some(Self::Caption),
			name => name
				.strip_prefix("heading ")
				.and_then(|level| level.parse::<u8>().ok())
				.filter(|level| (1..=6).contains(level))
				.map(Self::Heading),
		}
	}

	/// The kind of a built-in style by its English id, ie `Heading1`.
	fn from_id(id: &str) -> Option<Self> {
		match id {
			"Title" => Some(Self::Heading(1)),
			"Quote" | "IntenseQuote" => Some(Self::Quote),
			"HTMLPreformatted" => Some(Self::Preformatted),
			"HTMLCode" => Some(Self::Code),
			"Caption" => Some(Self::Caption),
			id => id
				.strip_prefix("Heading")
				.and_then(|level| level.parse::<u8>().ok())
				.filter(|level| (1..=6).contains(level))
				.map(Self::Heading),
		}
	}
}

impl WordNumbering {
	/// The numbering part: each list's levels and whether each counts.
	fn parse(nodes: &[BsxNode]) -> Self {
		let mut numbering = Self::default();
		let Some(root) = BsxNode::document_element(nodes) else {
			return numbering;
		};
		let levels = |parent: &BsxElement| {
			parent
				.children_named("lvl")
				.filter_map(|level| {
					Some((
						level.attribute_local("ilvl")?.parse::<u8>().ok()?,
						Self::counts(level),
					))
				})
				.collect::<Vec<_>>()
		};
		let abstracts = root
			.children_named("abstractNum")
			.filter_map(|item| {
				Some((
					SmolStr::new(item.attribute_local("abstractNumId")?),
					levels(item),
				))
			})
			.collect::<HashMap<_, _>>();
		for num in root.children_named("num") {
			let Some(id) = num.attribute_local("numId").map(SmolStr::new)
			else {
				continue;
			};
			let base = num
				.child("abstractNumId")
				.and_then(|base| base.attribute_local("val"))
				.and_then(|base| abstracts.get(base));
			for (level, ordered) in base.into_iter().flatten() {
				numbering.ordered.insert((id.clone(), *level), *ordered);
			}
			// a level override's own format wins
			for (level, ordered) in
				num.children_named("lvlOverride").flat_map(&levels)
			{
				numbering.ordered.insert((id.clone(), level), ordered);
			}
		}
		numbering
	}

	/// Whether a level counts, its `w:numFmt` anything but a bullet or none.
	fn counts(level: &BsxElement) -> bool {
		level
			.child("numFmt")
			.and_then(|format| format.attribute_local("val"))
			.is_some_and(|format| !matches!(format, "bullet" | "none"))
	}
}

impl WordRead<'_> {
	/// A paragraph properties element's numbering, its `numId` and `ilvl`,
	/// none for `numId` 0, which ends a style's numbering.
	fn numbering_of(properties: &BsxElement) -> Option<(SmolStr, u8)> {
		let numbering = properties.child("numPr")?;
		let num = numbering
			.child("numId")
			.and_then(|num| num.attribute_local("val"))
			.unwrap_or("0");
		let level = numbering
			.child("ilvl")
			.and_then(|level| level.attribute_local("val"))
			.and_then(|level| level.parse().ok())
			.unwrap_or(0);
		(num != "0").then(|| (SmolStr::new(num), level))
	}

	/// The part at `path`, a header or footer, as one `tag` element of its
	/// blocks.
	fn part_element(
		&mut self,
		path: &str,
		tag: &str,
	) -> Result<Option<ReadElement>> {
		let Some(nodes) = self.package.xml(path)? else {
			return Ok(None);
		};
		let Some(root) = BsxNode::document_element(&nodes) else {
			return Ok(None);
		};
		let mut element = ReadElement::new(tag);
		for block in self.blocks(path, root) {
			element = element.with_child(block);
		}
		Some(element).xok()
	}

	/// Each note referenced, in reference order, as a definition after the
	/// body: footnotes as `fn-<n>`, then endnotes as `en-<n>`.
	fn notes(&mut self, main: &str) -> Result<Vec<ReadElement>> {
		let mut out = Vec::new();
		for (kind, element, prefix) in [
			(OpcRelationship::FOOTNOTES, "footnote", "fn"),
			(OpcRelationship::ENDNOTES, "endnote", "en"),
		] {
			let Some(path) =
				self.package.related(Some(main), kind).map(str::to_string)
			else {
				continue;
			};
			let Some(nodes) = self.package.xml(&path)? else {
				continue;
			};
			let Some(root) = BsxNode::document_element(&nodes) else {
				continue;
			};
			// a note may reference another, so the list grows as it is read
			let mut index = 0;
			while let Some(id) = self.referenced(prefix).get(index).cloned() {
				index += 1;
				let Some(note) = root.children_named(element).find(|note| {
					note.attribute_local("id") == Some(id.as_str())
				}) else {
					continue;
				};
				self.note_start = true;
				let mut definition = ReadElement::new("div")
					.with_attribute("class", "footnote-definition")
					.with_attribute("id", format!("{prefix}-{index}"));
				for block in self.blocks(&path, note) {
					definition = definition.with_child(block);
				}
				self.note_start = false;
				out.push(definition);
			}
		}
		out.xok()
	}

	/// The notes of `prefix`'s kind referenced so far.
	fn referenced(&self, prefix: &str) -> &[SmolStr] {
		match prefix {
			"fn" => &self.footnotes,
			_ => &self.endnotes,
		}
	}

	/// The blocks of `container`, a body, cell, note, text box or header,
	/// of the part at `part`, grouped into lists, quotes and code.
	fn blocks(
		&mut self,
		part: &str,
		container: &BsxElement,
	) -> Vec<ReadElement> {
		let mut items = Vec::new();
		let mut pending = None;
		self.block_items(part, container, &mut items, &mut pending);
		Self::group(items)
	}

	/// Reads the block children of `container` into `items`, a bookmark
	/// between blocks naming the next as `pending`.
	fn block_items(
		&mut self,
		part: &str,
		container: &BsxElement,
		items: &mut Vec<BlockRead>,
		pending: &mut Option<SmolStr>,
	) {
		for child in container.elements() {
			let start = items.len();
			match child.local_name() {
				"p" => self.paragraph(part, child, items),
				"tbl" => items.push(BlockRead::Other(self.table(part, child))),
				"bookmarkStart" => {
					if pending.is_none() {
						*pending = Self::bookmark(child);
					}
				}
				// a block content control, custom markup or a revised
				// range: its blocks in place
				"sdt" => {
					if let Some(content) = child.child("sdtContent") {
						self.block_items(part, content, items, pending);
					}
				}
				"customXml" | "ins" | "moveTo" => {
					self.block_items(part, child, items, pending)
				}
				"AlternateContent" => {
					if let Some(branch) = Self::alternate(child) {
						self.block_items(part, branch, items, pending);
					}
				}
				_ => {}
			}
			// a bookmark before a block names it
			if items.len() > start
				&& let Some(id) = pending.take()
			{
				let element = match &mut items[start] {
					BlockRead::Paragraph { element, .. } => element,
					BlockRead::Other(element) => element,
				};
				if !element.attributes.iter().any(|(key, _)| key == "id") {
					element
						.attributes
						.push(("id".into(), Value::Str(id.into())));
				}
			}
		}
	}

	/// The id a bookmark gives its block, none for Word's own `_GoBack`.
	fn bookmark(start: &BsxElement) -> Option<SmolStr> {
		start
			.attribute_local("name")
			.filter(|name| !name.is_empty() && *name != "_GoBack")
			.map(SmolStr::new)
	}

	/// The branch of an `mc:AlternateContent` the reader reads: its first
	/// choice requiring only what it reads, else its fallback.
	fn alternate(content: &BsxElement) -> Option<&BsxElement> {
		content
			.children_named("Choice")
			.find(|choice| {
				choice.attribute_local("Requires").is_some_and(|requires| {
					requires
						.split_whitespace()
						.all(|prefix| WordParser::CHOICES.contains(&prefix))
				})
			})
			.or_else(|| content.child("Fallback"))
	}

	/// Reads a paragraph into `items`: itself by its kind, split at each page
	/// break, or an `hr`, or the text boxes it holds alone, each text box it
	/// holds after it.
	fn paragraph(
		&mut self,
		part: &str,
		paragraph: &BsxElement,
		items: &mut Vec<BlockRead>,
	) {
		let properties = paragraph.child("pPr");
		let style = properties
			.and_then(|properties| properties.child("pStyle"))
			.and_then(|style| style.attribute_local("val"));
		let numbering = properties.and_then(Self::numbering_of).or_else(|| {
			style.and_then(|style| self.styles.numbering.get(style).cloned())
		});
		let kind = match (
			style.and_then(|style| self.styles.kind(style)),
			numbering,
		) {
			(Some(StyleKind::Heading(level)), _) => {
				ParagraphKind::Heading(level)
			}
			(Some(StyleKind::Quote), _) => ParagraphKind::Quote,
			(Some(StyleKind::Preformatted), _) => ParagraphKind::Preformatted,
			(Some(StyleKind::Caption), _) => ParagraphKind::Caption,
			(_, Some((num, level))) => ParagraphKind::Item {
				level,
				ordered: self
					.numbering
					.ordered
					.get(&(num.clone(), level))
					.copied()
					.unwrap_or(false),
				num,
			},
			_ => ParagraphKind::Plain,
		};
		let mut inline = InlineRead {
			base_size: self.styles.size_of(style),
			..default()
		};
		self.inline_children(part, paragraph, &mut inline);
		let has_content = inline
			.segments
			.iter()
			.any(|segment| !matches!(segment.leaf, Leaf::PageBreak));
		let has_break = inline
			.segments
			.iter()
			.any(|segment| matches!(segment.leaf, Leaf::PageBreak));
		if properties
			.and_then(|properties| properties.child("pageBreakBefore"))
			.is_some_and(Self::toggle)
		{
			items.push(BlockRead::Other(Self::page_break()));
		}
		// an empty paragraph with only a bottom border is a rule
		if !has_content && !has_break && Self::is_rule(properties) {
			items.push(BlockRead::Other(ReadElement::new("hr")));
			return;
		}
		// a paragraph holding only text boxes is the boxes
		if !has_content && !has_break && !inline.asides.is_empty() {
			items.extend(inline.asides.into_iter().map(BlockRead::Other));
			return;
		}
		let look = DirectLook {
			text_align: properties
				.and_then(|properties| properties.child("jc"))
				.and_then(|align| align.attribute_local("val"))
				.and_then(|align| match align {
					"center" => Some(TextAlign::Center),
					"right" | "end" => Some(TextAlign::Right),
					_ => None,
				}),
			background: properties
				.and_then(|properties| properties.child("shd"))
				.and_then(Self::shading),
			..default()
		};
		let tag = match &kind {
			ParagraphKind::Heading(level) => format!("h{level}"),
			ParagraphKind::Item { .. } => "li".into(),
			ParagraphKind::Preformatted => "pre".into(),
			_ => "p".into(),
		};
		let mut bookmarks = inline.bookmarks.into_iter();
		let chunks = inline
			.segments
			.split(|segment| matches!(segment.leaf, Leaf::PageBreak))
			.collect::<Vec<_>>();
		for (index, chunk) in chunks.iter().enumerate() {
			if index > 0 {
				items.push(BlockRead::Other(Self::page_break()));
			}
			// a page break leaves no empty paragraph beside it
			if chunk.is_empty() && has_break {
				continue;
			}
			let mut element =
				ReadElement::new(tag.as_str()).with_look(look.clone());
			if let Some(id) = bookmarks.next() {
				element = element.with_attribute("id", id.to_string());
			}
			element.children = Self::assemble(chunk, 0);
			items.push(BlockRead::Paragraph {
				kind: kind.clone(),
				element,
			});
		}
		items.extend(inline.asides.into_iter().map(BlockRead::Other));
	}

	/// beet's `PageBreak` markup.
	fn page_break() -> ReadElement {
		ReadElement::new("div").with_attribute("class", "page-break")
	}

	/// Whether a paragraph's borders are a bottom border alone.
	fn is_rule(properties: Option<&BsxElement>) -> bool {
		properties
			.and_then(|properties| properties.child("pBdr"))
			.is_some_and(|borders| {
				let drawn = |side: &str| {
					borders.child(side).is_some_and(|border| {
						!matches!(
							border.attribute_local("val"),
							Some("nil" | "none")
						)
					})
				};
				drawn("bottom")
					&& !["top", "left", "right", "between"]
						.into_iter()
						.any(drawn)
			})
	}

	/// Whether a toggle property is on, its `w:val` anything but off.
	fn toggle(element: &BsxElement) -> bool {
		!matches!(element.attribute_local("val"), Some("0" | "false" | "off"))
	}

	/// A shading's fill, none for `auto`, by its theme slot when it names one.
	fn shading(shading: &BsxElement) -> Option<OfficeColor> {
		Self::color(
			shading.attribute_local("fill"),
			shading.attribute_local("themeFill"),
			shading.attribute_local("themeFillTint"),
			shading.attribute_local("themeFillShade"),
		)
	}

	/// A Word colour: its theme slot, tinted or shaded, else its value, none
	/// for `auto`.
	fn color(
		value: Option<&str>,
		theme: Option<&str>,
		tint: Option<&str>,
		shade: Option<&str>,
	) -> Option<OfficeColor> {
		let fraction = |hex: Option<&str>| {
			hex.and_then(|hex| u8::from_str_radix(hex, 16).ok())
				.map(|value| value as f32 / 255.)
		};
		if let Some(slot) = theme.and_then(ThemeSlot::parse) {
			let tint = match (fraction(tint), fraction(shade)) {
				(Some(tint), _) => 1. - tint,
				(None, Some(shade)) => shade - 1.,
				(None, None) => 0.,
			};
			return Some(OfficeColor::Slot { slot, tint });
		}
		value
			.filter(|value| !value.eq_ignore_ascii_case("auto"))
			.and_then(OfficeColor::parse_hex)
			.map(OfficeColor::Rgb)
	}

	/// Reads the inline children of `parent`, a paragraph or any element
	/// wrapping runs, into `inline`.
	fn inline_children(
		&mut self,
		part: &str,
		parent: &BsxElement,
		inline: &mut InlineRead,
	) {
		for child in parent.elements() {
			match child.local_name() {
				"r" => self.run(part, child, inline),
				"hyperlink" => {
					let href = self.hyperlink(part, child);
					self.wrapped(part, child, href, inline);
				}
				"ins" | "moveTo" => {
					let ins = Self::revision("ins", child);
					self.wrapped(part, child, Some(ins), inline);
				}
				"del" | "moveFrom" => {
					let del = Self::revision("del", child);
					self.wrapped(part, child, Some(del), inline);
				}
				"fldSimple" => {
					let link = child
						.attribute_local("instr")
						.and_then(Self::field_link);
					self.wrapped(part, child, link, inline);
				}
				"sdt" => match Self::checkbox(child) {
					Some(checkbox) => {
						inline.push(Leaf::Element(checkbox), Vec::new())
					}
					None => {
						if let Some(content) = child.child("sdtContent") {
							self.inline_children(part, content, inline);
						}
					}
				},
				"smartTag" | "customXml" | "dir" | "bdo" => {
					self.inline_children(part, child, inline)
				}
				"bookmarkStart" => {
					inline.bookmarks.extend(Self::bookmark(child))
				}
				"AlternateContent" => {
					if let Some(branch) = Self::alternate(child) {
						self.inline_children(part, branch, inline);
					}
				}
				_ => {}
			}
		}
	}

	/// Reads `parent`'s inline children inside `wrapper`, when it has one.
	fn wrapped(
		&mut self,
		part: &str,
		parent: &BsxElement,
		wrapper: Option<ReadElement>,
		inline: &mut InlineRead,
	) {
		let wrapped = wrapper
			.map(|wrapper| inline.context.push(wrapper))
			.is_some();
		self.inline_children(part, parent, inline);
		if wrapped {
			inline.context.pop();
		}
	}

	/// The `a` a hyperlink reads as: its relationship's target, its anchor
	/// after a `#`.
	fn hyperlink(&self, part: &str, link: &BsxElement) -> Option<ReadElement> {
		let target = link
			.prefixed_attribute("id")
			.and_then(|id| self.package.relationship(Some(part), id))
			.map(|relationship| relationship.target.to_string());
		let href = match (target, link.attribute_local("anchor")) {
			(Some(target), Some(anchor)) => format!("{target}#{anchor}"),
			(Some(target), None) => target,
			(None, Some(anchor)) => format!("#{anchor}"),
			(None, None) => return None,
		};
		Some(ReadElement::new("a").with_attribute("href", href))
	}

	/// The `ins` or `del` a revision reads as, its date its `datetime`.
	fn revision(tag: &str, revision: &BsxElement) -> ReadElement {
		let mut element = ReadElement::new(tag);
		if let Some(date) = revision.attribute_local("date") {
			element = element.with_attribute("datetime", date.to_string());
		}
		element
	}

	/// The `a` a `HYPERLINK` field instruction reads as, its `\l` anchor
	/// after a `#`.
	fn field_link(instruction: &str) -> Option<ReadElement> {
		let rest = instruction.trim().strip_prefix("HYPERLINK")?;
		let mut target = None;
		let mut anchor = None;
		let mut words = Self::field_words(rest).into_iter();
		while let Some(word) = words.next() {
			match word.as_str() {
				"\\l" => anchor = words.next(),
				switch if switch.starts_with('\\') => {
					// a switch taking an argument
					if matches!(switch, "\\o" | "\\t") {
						words.next();
					}
				}
				_ if target.is_none() => target = Some(word),
				_ => {}
			}
		}
		let href = match (target, anchor) {
			(Some(target), Some(anchor)) => format!("{target}#{anchor}"),
			(Some(target), None) => target,
			(None, Some(anchor)) => format!("#{anchor}"),
			(None, None) => return None,
		};
		Some(ReadElement::new("a").with_attribute("href", href))
	}

	/// A field instruction's words, a quoted one whole.
	fn field_words(instruction: &str) -> Vec<String> {
		let mut words = Vec::new();
		let mut chars = instruction.chars().peekable();
		while let Some(char) = chars.next() {
			match char {
				'"' => words.push(
					chars.by_ref().take_while(|char| *char != '"').collect(),
				),
				char if char.is_whitespace() => {}
				char => {
					let mut word = String::from(char);
					while let Some(next) =
						chars.next_if(|next| !next.is_whitespace())
					{
						word.push(next);
					}
					words.push(word);
				}
			}
		}
		words
	}

	/// A checkbox content control as the `input` it is, its glyph dropped.
	fn checkbox(control: &BsxElement) -> Option<ReadElement> {
		let checkbox = control.child("sdtPr")?.child("checkbox")?;
		let checked = checkbox
			.child("checked")
			.and_then(|checked| checked.attribute_local("val"))
			.is_some_and(|value| matches!(value, "1" | "true"));
		let mut input =
			ReadElement::new("input").with_attribute("type", "checkbox");
		if checked {
			input = input.with_flag("checked");
		}
		Some(input)
	}

	/// Reads one run: its look's wrappers around each piece it holds, a
	/// field's instruction skipped and its result read.
	fn run(&mut self, part: &str, run: &BsxElement, inline: &mut InlineRead) {
		let wrappers = run
			.child("rPr")
			.map(|properties| self.run_wrappers(properties, inline.base_size))
			.unwrap_or_default();
		self.run_content(part, run, &wrappers, inline);
	}

	/// Reads the content of `run`, or of an `mc:AlternateContent` branch in
	/// it, inside `wrappers`.
	fn run_content(
		&mut self,
		part: &str,
		run: &BsxElement,
		wrappers: &[ReadElement],
		inline: &mut InlineRead,
	) {
		for child in run.elements() {
			let local = child.local_name();
			// a field's instruction is read for its kind, never shown
			match local {
				"fldChar" => {
					match child.attribute_local("fldCharType") {
						Some("begin") => inline.fields.push(Field::default()),
						Some("separate") => {
							if let Some(field) = inline.fields.last_mut() {
								field.in_result = true;
								field.link =
									Self::field_link(&field.instruction);
							}
						}
						Some("end") => {
							inline.fields.pop();
						}
						_ => {}
					}
					continue;
				}
				"instrText" | "delInstrText" => {
					if let Some(field) = inline.fields.last_mut() {
						field.instruction.push_str(&child.text());
					}
					continue;
				}
				_ => {}
			}
			if inline.fields.iter().any(|field| !field.in_result) {
				continue;
			}
			let text = |text: String| Leaf::Text(text);
			let leaf = match local {
				"t" | "delText" => {
					let mut words = child.text();
					if self.note_start {
						words = words.trim_start().to_string();
						self.note_start = words.is_empty();
					}
					text(words)
				}
				"tab" | "ptab" => text(" ".into()),
				"noBreakHyphen" => text("\u{2011}".into()),
				"sym" => match child
					.attribute_local("char")
					.and_then(|code| u32::from_str_radix(code, 16).ok())
					.and_then(char::from_u32)
				{
					Some(symbol) => text(symbol.to_string()),
					None => continue,
				},
				"br" if child.attribute_local("type") == Some("page") => {
					Leaf::PageBreak
				}
				"br" | "cr" => Leaf::Element(ReadElement::new("br")),
				"footnoteReference" | "endnoteReference" => {
					let Some(id) =
						child.attribute_local("id").map(SmolStr::new)
					else {
						continue;
					};
					let (notes, prefix) = match local {
						"footnoteReference" => (&mut self.footnotes, "fn"),
						_ => (&mut self.endnotes, "en"),
					};
					let number =
						match notes.iter().position(|found| *found == id) {
							Some(index) => index + 1,
							None => {
								notes.push(id);
								notes.len()
							}
						};
					// the reference's own look is its style's superscript
					inline.push(
						Leaf::Element(
							ReadElement::new("sup").with_child(
								ReadElement::new("a")
									.with_attribute(
										"href",
										format!("#{prefix}-{number}"),
									)
									.with_text(number.to_string()),
							),
						),
						Vec::new(),
					);
					continue;
				}
				"drawing" => {
					self.drawing(part, child, wrappers, inline);
					continue;
				}
				"pict" => {
					self.picture(part, child, wrappers, inline);
					continue;
				}
				"AlternateContent" => {
					if let Some(branch) = Self::alternate(child) {
						self.run_content(part, branch, wrappers, inline);
					}
					continue;
				}
				_ => continue,
			};
			inline.push(leaf, wrappers.to_vec());
		}
	}

	/// The wrappers a run's properties make, outermost first: `mark` or a
	/// `span` carrying its direct look, `strong`, `em`, `u`, `s`, `sub` or
	/// `sup`, `code`.
	fn run_wrappers(
		&self,
		properties: &BsxElement,
		base_size: u32,
	) -> Vec<ReadElement> {
		let toggle = |local: &str| properties.child(local).map(Self::toggle);
		let highlight = properties
			.child("highlight")
			.and_then(|highlight| highlight.attribute_local("val"))
			.filter(|highlight| *highlight != "none");
		let look = DirectLook {
			color: properties.child("color").and_then(|color| {
				Self::color(
					color.attribute_local("val"),
					color.attribute_local("themeColor"),
					color.attribute_local("themeTint"),
					color.attribute_local("themeShade"),
				)
			}),
			background: match highlight {
				Some(name) => (name != WordHighlight::DEFAULT)
					.then(|| WordHighlight::color(name))
					.flatten()
					.map(OfficeColor::Rgb),
				None => properties.child("shd").and_then(Self::shading),
			},
			font_size_pt: WordStyles::size(properties)
				.filter(|size| *size != base_size)
				.map(|size| size as f32 / 2.),
			bold: toggle("b").filter(|bold| !bold),
			italic: toggle("i").filter(|italic| !italic),
			underline: properties
				.child("u")
				.map(|underline| {
					underline.attribute_local("val") != Some("none")
				})
				.filter(|underline| !underline),
			strike: toggle("strike")
				.or(toggle("dstrike"))
				.filter(|strike| !strike),
			text_align: None,
		};
		let mut wrappers = Vec::new();
		match (highlight.is_some(), look.is_empty()) {
			(true, _) => {
				wrappers.push(ReadElement::new("mark").with_look(look))
			}
			(false, false) => {
				wrappers.push(ReadElement::new("span").with_look(look))
			}
			(false, true) => {}
		}
		let underline = properties.child("u").is_some_and(|underline| {
			underline.attribute_local("val") != Some("none")
		});
		for (on, tag) in [
			(toggle("b") == Some(true), "strong"),
			(toggle("i") == Some(true), "em"),
			(underline, "u"),
			(
				toggle("strike") == Some(true)
					|| toggle("dstrike") == Some(true),
				"s",
			),
		] {
			if on {
				wrappers.push(ReadElement::new(tag));
			}
		}
		match properties
			.child("vertAlign")
			.and_then(|align| align.attribute_local("val"))
		{
			Some("superscript") => wrappers.push(ReadElement::new("sup")),
			Some("subscript") => wrappers.push(ReadElement::new("sub")),
			_ => {}
		}
		if properties
			.child("rStyle")
			.and_then(|style| style.attribute_local("val"))
			.and_then(|style| self.styles.kind(style))
			== Some(StyleKind::Code)
		{
			wrappers.push(ReadElement::new("code"));
		}
		wrappers
	}

	/// A DrawingML drawing's pictures as `img`s and its text boxes as
	/// `aside`s.
	fn drawing(
		&mut self,
		part: &str,
		drawing: &BsxElement,
		wrappers: &[ReadElement],
		inline: &mut InlineRead,
	) {
		let placed =
			drawing.child("inline").or_else(|| drawing.child("anchor"));
		let extent = placed
			.and_then(|placed| placed.child("extent"))
			.and_then(Self::extent);
		let alt = placed
			.and_then(|placed| placed.child("docPr"))
			.and_then(|properties| properties.attribute("descr"))
			.unwrap_or_default();
		let boxes = drawing.descendants_named("txbxContent");
		if !boxes.is_empty() {
			for content in boxes {
				let mut aside = ReadElement::new("aside");
				for block in self.blocks(part, content) {
					aside = aside.with_child(block);
				}
				inline.asides.push(aside);
			}
			return;
		}
		let pictures = drawing.descendants_named("pic");
		let single = pictures.len() == 1;
		for picture in pictures {
			let Some(blip) =
				picture.descendants_named("blip").into_iter().next()
			else {
				continue;
			};
			// a lone picture takes the drawing's extent, one of a group its own
			let size = match single {
				true => extent,
				false => picture
					.child("spPr")
					.and_then(|properties| properties.child("xfrm"))
					.and_then(|transform| transform.child("ext"))
					.and_then(Self::extent),
			};
			let img = self.blip(
				part,
				blip.prefixed_attribute("embed"),
				blip.prefixed_attribute("link"),
				size,
				alt,
			);
			if let Some(img) = img {
				inline.push(Leaf::Element(img), wrappers.to_vec());
			}
		}
	}

	/// A VML picture, Word's older drawing, its image as an `img` and its
	/// text box as an `aside`.
	fn picture(
		&mut self,
		part: &str,
		picture: &BsxElement,
		wrappers: &[ReadElement],
		inline: &mut InlineRead,
	) {
		for content in picture.descendants_named("txbxContent") {
			let mut aside = ReadElement::new("aside");
			for block in self.blocks(part, content) {
				aside = aside.with_child(block);
			}
			inline.asides.push(aside);
		}
		for shape in picture.descendants_named("shape") {
			let Some(image) = shape.child("imagedata") else {
				continue;
			};
			// the shape's size, `width:72pt;height:36pt`, in EMU
			let style = shape.attribute("style").unwrap_or_default();
			let side = |name: &str| {
				style.split(';').find_map(|declaration| {
					let (key, value) = declaration.split_once(':')?;
					(key.trim() == name)
						.then(|| {
							value.trim().strip_suffix("pt")?.parse::<f32>().ok()
						})
						.flatten()
				})
			};
			let size =
				side("width").zip(side("height")).map(|(width, height)| {
					UVec2::new(
						(width * 12700.) as u32,
						(height * 12700.) as u32,
					)
				});
			let alt = shape.attribute("alt").unwrap_or_default();
			if let Some(img) =
				self.blip(part, image.prefixed_attribute("id"), None, size, alt)
			{
				inline.push(Leaf::Element(img), wrappers.to_vec());
			}
		}
	}

	/// An `img` of the picture `embed` relates, its bytes as a `data:` URL,
	/// else of the address `link` relates.
	fn blip(
		&self,
		part: &str,
		embed: Option<&str>,
		link: Option<&str>,
		size: Option<UVec2>,
		alt: &str,
	) -> Option<ReadElement> {
		if let Some(relationship) =
			embed.and_then(|id| self.package.relationship(Some(part), id))
		{
			return match relationship.external {
				true => Some(OfficePicture::read(
					relationship.target.to_string(),
					size,
					alt,
				)),
				false => {
					let path = relationship.target.as_str();
					Some(OfficePicture::read_embedded(
						self.package.part(path)?,
						self.package.content_type(path)?,
						size,
						alt,
					))
				}
			};
		}
		let relationship =
			link.and_then(|id| self.package.relationship(Some(part), id))?;
		Some(OfficePicture::read(
			relationship.target.to_string(),
			size,
			alt,
		))
	}

	/// A DrawingML extent's `cx` and `cy` in EMU.
	fn extent(extent: &BsxElement) -> Option<UVec2> {
		Some(UVec2::new(
			extent.attribute("cx")?.parse().ok()?,
			extent.attribute("cy")?.parse().ok()?,
		))
	}

	/// A table: a leading run of header rows as a `thead` of `th`s, every
	/// other row in a `tbody`, a merge as spans on its first cell.
	fn table(&mut self, part: &str, table: &BsxElement) -> ReadElement {
		/// A cell as read, before its rows' merges are known.
		struct CellRead {
			column: u32,
			merge: Option<bool>,
			element: ReadElement,
		}
		let mut rows = Vec::<(bool, Vec<CellRead>)>::new();
		let mut heading = true;
		for row in Self::through_controls(table, "tr") {
			let properties = row.child("trPr");
			heading &= properties
				.and_then(|properties| properties.child("tblHeader"))
				.is_some_and(Self::toggle);
			let mut column = properties
				.and_then(|properties| properties.child("gridBefore"))
				.and_then(|before| before.attribute_local("val"))
				.and_then(|before| before.parse::<u32>().ok())
				.unwrap_or(0);
			let tag = if heading { "th" } else { "td" };
			// a skipped grid column is an empty cell, so slots stay addressed
			let mut cells = (0..column)
				.map(|index| CellRead {
					column: index,
					merge: None,
					element: ReadElement::new(tag),
				})
				.collect::<Vec<_>>();
			for cell in Self::through_controls(row, "tc") {
				let properties = cell.child("tcPr");
				let span = properties
					.and_then(|properties| properties.child("gridSpan"))
					.and_then(|span| span.attribute_local("val"))
					.and_then(|span| span.parse::<u32>().ok())
					.unwrap_or(1)
					.max(1);
				let merge = properties
					.and_then(|properties| properties.child("vMerge"))
					.map(|merge| {
						merge.attribute_local("val") == Some("restart")
					});
				let look = DirectLook {
					background: properties
						.and_then(|properties| properties.child("shd"))
						.and_then(Self::shading),
					..default()
				};
				let mut element = ReadElement::new(tag).with_look(look);
				let blocks = self.blocks(part, cell);
				element = Self::cell_content(element, blocks);
				if span > 1 {
					element =
						element.with_attribute("colspan", span.to_string());
				}
				cells.push(CellRead {
					column,
					merge,
					element,
				});
				column += span;
			}
			rows.push((heading, cells));
		}
		// a continuation is the merge above it carried down, its own cell absent
		let is_continuation =
			|rows: &[(bool, Vec<CellRead>)], row: usize, column: u32| {
				row > 0
					&& rows[row].1.iter().any(|cell| {
						cell.column == column && cell.merge == Some(false)
					}) && rows[row - 1]
					.1
					.iter()
					.any(|cell| cell.column == column && cell.merge.is_some())
			};
		let mut head = ReadElement::new("thead");
		let mut body = ReadElement::new("tbody");
		for index in 0..rows.len() {
			let mut tr = ReadElement::new("tr");
			for cell in &rows[index].1 {
				if is_continuation(&rows, index, cell.column) {
					continue;
				}
				let mut element = cell.element.clone();
				if cell.merge.is_some() {
					let down = (index + 1..rows.len())
						.take_while(|below| {
							is_continuation(&rows, *below, cell.column)
						})
						.count();
					if down > 0 {
						element = element
							.with_attribute("rowspan", (down + 1).to_string());
					}
				}
				tr = tr.with_child(element);
			}
			match rows[index].0 {
				true => head = head.with_child(tr),
				false => body = body.with_child(tr),
			}
		}
		let mut element = ReadElement::new("table");
		if !head.children.is_empty() {
			element = element.with_child(head);
		}
		if !body.children.is_empty() {
			element = element.with_child(body);
		}
		element
	}

	/// The children of `parent` named `local`, a content control's or custom
	/// markup's walked through.
	fn through_controls<'b>(
		parent: &'b BsxElement,
		local: &str,
	) -> Vec<&'b BsxElement> {
		let mut out = Vec::new();
		for child in parent.elements() {
			match child.local_name() {
				found if found == local => out.push(child),
				"sdt" => {
					if let Some(content) = child.child("sdtContent") {
						out.extend(Self::through_controls(content, local));
					}
				}
				"customXml" => out.extend(Self::through_controls(child, local)),
				_ => {}
			}
		}
		out
	}

	/// A cell holding `blocks`: a lone plain paragraph's content held
	/// directly, its look and id the cell's, else the blocks, the empty
	/// paragraph Word requires after a cell's last table dropped.
	fn cell_content(
		mut cell: ReadElement,
		mut blocks: Vec<ReadElement>,
	) -> ReadElement {
		let closes_table = match blocks.as_slice() {
			[.., table, last] => {
				table.tag == "table"
					&& last.tag == "p"
					&& last.children.is_empty()
					&& last.attributes.is_empty()
					&& last.look.is_empty()
			}
			_ => false,
		};
		if closes_table {
			blocks.pop();
		}
		match <[ReadElement; 1]>::try_from(blocks) {
			Ok([paragraph]) if paragraph.tag == "p" => {
				let look = &paragraph.look;
				cell.look.text_align = look.text_align;
				cell.look.background = cell.look.background.or(look.background);
				cell.attributes.extend(paragraph.attributes);
				cell.children = paragraph.children;
				cell
			}
			Ok(blocks) => {
				blocks.into_iter().fold(cell, ReadElement::with_child)
			}
			Err(blocks) => {
				blocks.into_iter().fold(cell, ReadElement::with_child)
			}
		}
	}

	/// Groups each run of list items into nested lists, of quote paragraphs
	/// into a `blockquote` and of preformatted lines into one `pre`, and
	/// lays a caption directly above a table into it.
	fn group(items: Vec<BlockRead>) -> Vec<ReadElement> {
		let mut out = Vec::new();
		let mut items = items.into_iter().peekable();
		while let Some(item) = items.next() {
			let (kind, element) = match item {
				BlockRead::Other(element) => {
					out.push(element);
					continue;
				}
				BlockRead::Paragraph { kind, element } => (kind, element),
			};
			match kind {
				ParagraphKind::Item { .. } => {
					let mut run = vec![(kind, element)];
					while let Some(BlockRead::Paragraph {
						kind: ParagraphKind::Item { .. },
						..
					}) = items.peek()
					{
						let Some(BlockRead::Paragraph { kind, element }) =
							items.next()
						else {
							break;
						};
						run.push((kind, element));
					}
					out.extend(Self::lists(run));
				}
				ParagraphKind::Quote => {
					let mut quote =
						ReadElement::new("blockquote").with_child(element);
					while let Some(BlockRead::Paragraph {
						kind: ParagraphKind::Quote,
						..
					}) = items.peek()
					{
						let Some(BlockRead::Paragraph { element, .. }) =
							items.next()
						else {
							break;
						};
						quote = quote.with_child(element);
					}
					out.push(quote);
				}
				ParagraphKind::Preformatted => {
					let mut lines = vec![element];
					while let Some(BlockRead::Paragraph {
						kind: ParagraphKind::Preformatted,
						..
					}) = items.peek()
					{
						let Some(BlockRead::Paragraph { element, .. }) =
							items.next()
						else {
							break;
						};
						lines.push(element);
					}
					out.push(Self::preformatted(lines));
				}
				ParagraphKind::Caption => match items.peek() {
					Some(BlockRead::Other(table)) if table.tag == "table" => {
						let Some(BlockRead::Other(mut table)) = items.next()
						else {
							continue;
						};
						let mut caption =
							ReadElement::new("caption").with_look(element.look);
						caption.attributes = element.attributes;
						caption.children = element.children;
						table.children.insert(0, ReadNode::Element(caption));
						out.push(table);
					}
					_ => out.push(element),
				},
				_ => out.push(element),
			}
		}
		out
	}

	/// A run of list items as lists, a top level item of another list
	/// starting a new one.
	fn lists(items: Vec<(ParagraphKind, ReadElement)>) -> Vec<ReadElement> {
		let items = items
			.into_iter()
			.filter_map(|(kind, element)| match kind {
				ParagraphKind::Item {
					level,
					ordered,
					num,
				} => Some((level, ordered, num, element)),
				_ => None,
			})
			.collect::<Vec<_>>();
		let top = items.iter().map(|item| item.0).min().unwrap_or(0);
		let mut out = Vec::new();
		let mut start = 0;
		for index in 1..=items.len() {
			let ends = index == items.len()
				|| (items[index].0 == top && items[index].2 != items[start].2);
			if ends {
				out.push(Self::list(&items[start..index], top));
				start = index;
			}
		}
		out
	}

	/// One list at `depth` of `items`, a deeper item nested in the item
	/// before it.
	fn list(
		items: &[(u8, bool, SmolStr, ReadElement)],
		depth: u8,
	) -> ReadElement {
		let tag = if items[0].1 { "ol" } else { "ul" };
		let mut list = ReadElement::new(tag);
		let mut index = 0;
		while index < items.len() {
			if items[index].0 <= depth {
				list = list.with_child(items[index].3.clone());
				index += 1;
				continue;
			}
			let end = (index..items.len())
				.find(|end| items[*end].0 <= depth)
				.unwrap_or(items.len());
			let nested = Self::list(&items[index..end], depth + 1);
			match list
				.children
				.iter_mut()
				.rev()
				.find_map(|child| match child {
					ReadNode::Element(element) => Some(element),
					ReadNode::Text(_) => None,
				}) {
				Some(last) => last.children.push(ReadNode::Element(nested)),
				None => {
					list = list
						.with_child(ReadElement::new("li").with_child(nested))
				}
			}
			index = end;
		}
		list
	}

	/// Preformatted lines as one `pre`, a line each, lines each wholly in
	/// one `code` held in one.
	fn preformatted(lines: Vec<ReadElement>) -> ReadElement {
		let mut pre = ReadElement::new("pre").with_look(lines[0].look.clone());
		pre.attributes = lines[0].attributes.clone();
		let coded = lines.iter().all(|line| match line.children.as_slice() {
			[] => true,
			[ReadNode::Element(element)] => {
				element.tag == "code"
					&& element.attributes.is_empty()
					&& element.look.is_empty()
			}
			_ => false,
		});
		let mut joined = Vec::new();
		for (index, line) in lines.into_iter().enumerate() {
			if index > 0 {
				joined.push(ReadNode::Text("\n".into()));
			}
			match coded {
				true => {
					joined.extend(line.children.into_iter().flat_map(|child| {
						match child {
							ReadNode::Element(code) => code.children,
							text => vec![text],
						}
					}))
				}
				false => joined.extend(line.children),
			}
		}
		let joined = ReadNode::merge_text(joined);
		match coded {
			true => {
				let mut code = ReadElement::new("code");
				code.children = joined;
				pre.with_child(code)
			}
			false => {
				pre.children = joined;
				pre
			}
		}
	}

	/// The content of `segments` from their wrappers at `depth` in, each run
	/// of equal wrappers one element.
	fn assemble(segments: &[Segment], depth: usize) -> Vec<ReadNode> {
		let mut out = Vec::new();
		let mut index = 0;
		while index < segments.len() {
			let segment = &segments[index];
			let Some(wrapper) = segment.wrappers.get(depth) else {
				match &segment.leaf {
					Leaf::Text(text) => out.push(ReadNode::Text(text.clone())),
					Leaf::Element(element) => {
						out.push(ReadNode::Element(element.clone()))
					}
					Leaf::PageBreak => {}
				}
				index += 1;
				continue;
			};
			let end = (index..segments.len())
				.find(|end| {
					segments[*end]
						.wrappers
						.get(depth)
						.is_none_or(|other| !wrapper.same_shell(other))
				})
				.unwrap_or(segments.len());
			let mut element = wrapper.clone();
			element.children = Self::assemble(&segments[index..end], depth + 1);
			out.push(ReadNode::Element(element));
			index = end;
		}
		ReadNode::merge_text(out)
	}
}

impl InlineRead {
	/// Adds `leaf` inside the open context, then `wrappers`.
	fn push(&mut self, leaf: Leaf, wrappers: Vec<ReadElement>) {
		let links = self.fields.iter().filter_map(|field| field.link.clone());
		self.segments.push(Segment {
			wrappers: self
				.context
				.iter()
				.cloned()
				.chain(links)
				.chain(wrappers)
				.collect(),
			leaf,
		});
	}
}

impl Installer for WordParser {
	fn install(self, app: &mut App) { DocParsers::register_on(app, self); }
}

impl DocParser for WordParser {
	fn media_types(&self) -> Vec<MediaType> { vec![MediaType::Docx] }

	/// The [`DocMeta`] a Word file's core properties declare, named as
	/// `frontmatter_type`.
	fn declarations(
		&self,
		bytes: &MediaBytes,
		frontmatter_type: &str,
	) -> Result<RootDeclarations> {
		DocMeta::from_package(&OpcPackage::read(bytes.bytes())?)?
			.declarations(frontmatter_type)
			.xok()
	}

	fn parse(&mut self, cx: ParseContext) -> Result<(), ParseError> {
		if cx.bytes.media_type() != &MediaType::Docx {
			return Err(ParseError::UnsupportedType {
				unsupported: cx.bytes.media_type().clone(),
				supported: self.media_types(),
			});
		}
		let package = OpcPackage::read(cx.bytes.bytes())?;
		let meta = DocMeta::from_package(&package)?;
		if meta != DocMeta::default() {
			cx.entity.insert(meta);
		}
		let root = cx.entity.id();
		cx.entity
			.world_scope(|world| self.read(world, root, &package))?;
		Ok(())
	}
}

#[cfg(test)]
pub(crate) mod test {
	use super::super::office_test::*;
	use super::*;
	use crate::style::*;

	/// A synthetic Word file built over [`OpcPackage`], the one fixture the
	/// reader's and writer's tests share: a body with every namespace a
	/// reader reads bound, its styles, numbering, notes, header, footer,
	/// relationships and core properties.
	#[derive(Default)]
	pub(crate) struct WordFixture {
		body: String,
		styles: Option<String>,
		numbering: Option<String>,
		footnotes: Vec<String>,
		endnotes: Vec<String>,
		header: Option<String>,
		footer: Option<String>,
		/// Each external hyperlink's id and target, in id order.
		links: Vec<(String, String)>,
		/// Each picture's id, content type and bytes, after the links.
		images: Vec<(String, String, Vec<u8>)>,
		properties: Option<String>,
	}

	impl WordFixture {
		/// The prefixes a fixture's parts bind.
		const NAMESPACES: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
			xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
			xmlns:wp=\"http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing\" \
			xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
			xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\" \
			xmlns:wps=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\" \
			xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
			xmlns:w14=\"http://schemas.microsoft.com/office/word/2010/wordml\" \
			xmlns:v=\"urn:schemas-microsoft-com:vml\"";

		/// A file whose body is `body`.
		pub fn new(body: &str) -> Self {
			Self {
				body: body.into(),
				..default()
			}
		}

		/// Sets the styles part's inner elements.
		pub fn with_styles(mut self, xml: &str) -> Self {
			self.styles = Some(xml.into());
			self
		}

		/// Sets the numbering part's inner elements.
		pub fn with_numbering(mut self, xml: &str) -> Self {
			self.numbering = Some(xml.into());
			self
		}

		/// Adds a footnote of `xml` blocks, its id the count before it plus 1.
		pub fn with_footnote(mut self, xml: &str) -> Self {
			self.footnotes.push(xml.into());
			self
		}

		/// Adds an endnote of `xml` blocks.
		pub fn with_endnote(mut self, xml: &str) -> Self {
			self.endnotes.push(xml.into());
			self
		}

		/// Sets the default header's blocks.
		pub fn with_header(mut self, xml: &str) -> Self {
			self.header = Some(xml.into());
			self
		}

		/// Sets the default footer's blocks.
		pub fn with_footer(mut self, xml: &str) -> Self {
			self.footer = Some(xml.into());
			self
		}

		/// Relates the document to the external `target` by `id`, the ids
		/// counted `rId1`, `rId2`, .. across links then pictures.
		pub fn with_link(mut self, id: &str, target: &str) -> Self {
			self.links.push((id.into(), target.into()));
			self
		}

		/// Relates the document to a picture of `content_type` by `id`.
		pub fn with_image(
			mut self,
			id: &str,
			content_type: &str,
			bytes: Vec<u8>,
		) -> Self {
			self.images.push((id.into(), content_type.into(), bytes));
			self
		}

		/// Sets the core properties' inner elements, `dc:` bound.
		pub fn with_properties(mut self, xml: &str) -> Self {
			self.properties = Some(xml.into());
			self
		}

		/// The file's bytes.
		pub fn bytes(self) -> MediaBytes {
			let namespaces = Self::NAMESPACES;
			let main = "word/document.xml";
			let mut package = OpcPackage::default();
			// the fixture names its own ids, so they are added first, in order
			for (id, target) in &self.links {
				let added = package.relate_external(
					Some(main),
					OpcRelationship::HYPERLINK,
					target,
				);
				assert_eq!(&added, id, "fixture ids are `rId1`, `rId2`, ..");
			}
			for (index, (id, content_type, bytes)) in
				self.images.iter().enumerate()
			{
				let path = format!(
					"word/media/image{}.{}",
					index + 1,
					OfficePicture::extension(content_type)
				);
				package.insert(
					path.as_str(),
					content_type.as_str(),
					bytes.clone(),
				);
				let added =
					package.relate(Some(main), OpcRelationship::IMAGE, &path);
				assert_eq!(&added, id, "fixture ids are `rId1`, `rId2`, ..");
			}
			let mut section = String::new();
			for (xml, name, root, reference, kind) in [
				(
					&self.header,
					"header",
					"w:hdr",
					"w:headerReference",
					OpcRelationship::HEADER,
				),
				(
					&self.footer,
					"footer",
					"w:ftr",
					"w:footerReference",
					OpcRelationship::FOOTER,
				),
			] {
				let Some(xml) = xml else {
					continue;
				};
				let path = format!("word/{name}1.xml");
				package.insert(
					path.as_str(),
					OpcPackage::XML_TYPE,
					format!("<{root} {namespaces}>{xml}</{root}>"),
				);
				let id = package.relate(Some(main), kind, &path);
				section.push_str(&format!(
					"<{reference} w:type=\"default\" r:id=\"{id}\"/>"
				));
			}
			package.insert(
				main,
				"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
				format!(
					"<w:document {namespaces}><w:body>{}<w:sectPr>{section}</w:sectPr></w:body></w:document>",
					self.body
				),
			);
			package.relate(None, OpcRelationship::OFFICE_DOCUMENT, main);
			for (xml, path, root, kind) in [
				(
					&self.styles,
					"word/styles.xml",
					"w:styles",
					OpcRelationship::STYLES,
				),
				(
					&self.numbering,
					"word/numbering.xml",
					"w:numbering",
					OpcRelationship::NUMBERING,
				),
			] {
				if let Some(xml) = xml {
					package.insert(
						path,
						OpcPackage::XML_TYPE,
						format!("<{root} {namespaces}>{xml}</{root}>"),
					);
					package.relate(Some(main), kind, path);
				}
			}
			for (notes, element, path, kind) in [
				(
					&self.footnotes,
					"footnote",
					"word/footnotes.xml",
					OpcRelationship::FOOTNOTES,
				),
				(
					&self.endnotes,
					"endnote",
					"word/endnotes.xml",
					OpcRelationship::ENDNOTES,
				),
			] {
				if notes.is_empty() {
					continue;
				}
				let mut xml = format!(
					"<w:{element}s {namespaces}><w:{element} w:type=\"separator\" w:id=\"-1\"><w:p><w:r><w:separator/></w:r></w:p></w:{element}>"
				);
				for (index, note) in notes.iter().enumerate() {
					xml.push_str(&format!(
						"<w:{element} w:id=\"{}\">{note}</w:{element}>",
						index + 1
					));
				}
				xml.push_str(&format!("</w:{element}s>"));
				package.insert(path, OpcPackage::XML_TYPE, xml);
				package.relate(Some(main), kind, path);
			}
			if let Some(properties) = self.properties {
				package.insert(
					DocMeta::CORE_PROPERTIES_PATH,
					OpcPackage::CORE_PROPERTIES_TYPE,
					format!(
						"<cp:coreProperties xmlns:cp=\"{}\" xmlns:dc=\"{}\">{properties}</cp:coreProperties>",
						OoxmlNamespace::CORE_PROPERTIES,
						OoxmlNamespace::DUBLIN_CORE,
					),
				);
				package.relate(
					None,
					OpcRelationship::CORE_PROPERTIES,
					DocMeta::CORE_PROPERTIES_PATH,
				);
			}
			MediaBytes::new(MediaType::Docx, package.to_bytes().unwrap())
		}
	}

	/// A paragraph of one run of `text` with the run properties `properties`.
	pub(crate) fn run(properties: &str, text: &str) -> String {
		format!(
			"<w:r><w:rPr>{properties}</w:rPr><w:t xml:space=\"preserve\">{text}</w:t></w:r>"
		)
	}

	/// A paragraph styled `style` holding `content`.
	pub(crate) fn paragraph(style: &str, content: &str) -> String {
		match style {
			"" => format!("<w:p>{content}</w:p>"),
			style => format!(
				"<w:p><w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>{content}</w:p>"
			),
		}
	}

	/// A Word file covering every row of the module docs' Word mapping:
	/// styles matched by name under localized ids, runs of every look,
	/// links, fields, controls, revisions, a page break, a rule, lists,
	/// quotes, code, a captioned table of spans and merges, a picture, a
	/// text box, notes, a header and a footer.
	pub(crate) fn covering() -> MediaBytes {
		let plain = |text: &str| run("", text);
		let body = [
			format!(
				"<w:p><w:pPr><w:pStyle w:val=\"Title\"/></w:pPr><w:bookmarkStart w:id=\"0\" w:name=\"_GoBack\"/>\
				 <w:bookmarkStart w:id=\"1\" w:name=\"cover\"/>{}<w:bookmarkEnd w:id=\"1\"/></w:p>",
				plain("Cover")
			),
			paragraph("berschrift2", &plain("Looks")),
			paragraph(
				"",
				&[
					plain("plain "),
					run("<w:b/>", "bold"),
					run("<w:i/>", "italic"),
					run("<w:u w:val=\"single\"/>", "under"),
					run("<w:strike/>", "struck"),
					run("<w:dstrike/>", "double"),
					run("<w:vertAlign w:val=\"superscript\"/>", "up"),
					run("<w:vertAlign w:val=\"subscript\"/>", "down"),
					run("<w:color w:val=\"FF0000\"/>", "red"),
					run("<w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"00FFFF\"/>", "filled"),
					run("<w:sz w:val=\"28\"/>", "big"),
					run("<w:highlight w:val=\"yellow\"/>", "marked"),
					run("<w:highlight w:val=\"green\"/>", "green"),
					run("<w:rStyle w:val=\"HTMLCode\"/>", "code"),
					run("<w:b/><w:i/><w:color w:val=\"0000FF\"/>", "all"),
					"<w:r><w:t>a</w:t><w:tab/><w:t>b</w:t><w:sym w:font=\"Symbol\" w:char=\"03A9\"/><w:noBreakHyphen/><w:br/><w:t>after</w:t></w:r>".into(),
				]
				.concat(),
			),
			format!("<w:p><w:pPr><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"FFFF00\"/><w:jc w:val=\"center\"/></w:pPr>{}</w:p>", plain("centered")),
			paragraph(
				"",
				&[
					format!("<w:hyperlink r:id=\"rId1\">{}</w:hyperlink>", plain("site")),
					format!("<w:hyperlink w:anchor=\"cover\">{}</w:hyperlink>", plain("back")),
					"<w:r><w:fldChar w:fldCharType=\"begin\"/></w:r><w:r><w:instrText xml:space=\"preserve\"> HYPERLINK \"https://example.com/field\" </w:instrText></w:r>\
					 <w:r><w:fldChar w:fldCharType=\"separate\"/></w:r><w:r><w:t>field</w:t></w:r><w:r><w:fldChar w:fldCharType=\"end\"/></w:r>".into(),
					format!("<w:fldSimple w:instr=\" PAGE \">{}</w:fldSimple>", plain("3")),
				]
				.concat(),
			),
			paragraph(
				"",
				&[
					"<w:sdt><w:sdtPr><w14:checkbox><w14:checked w14:val=\"1\"/></w14:checkbox></w:sdtPr><w:sdtContent><w:r><w:t>\u{2612}</w:t></w:r></w:sdtContent></w:sdt>".into(),
					"<w:sdt><w:sdtPr><w:alias w:val=\"Name\"/></w:sdtPr><w:sdtContent>".to_string() + &plain(" Name") + "</w:sdtContent></w:sdt>",
				]
				.concat(),
			),
			paragraph(
				"",
				&format!(
					"<w:ins w:id=\"5\" w:author=\"Ada\" w:date=\"2024-03-01T00:00:00Z\">{}</w:ins>\
					 <w:del w:id=\"6\" w:author=\"Ada\" w:date=\"2024-03-02T00:00:00Z\"><w:r><w:delText>gone</w:delText></w:r></w:del>",
					plain("added")
				),
			),
			"<w:p><w:r><w:t>before</w:t><w:br w:type=\"page\"/><w:t>after</w:t></w:r></w:p>".into(),
			"<w:p><w:pPr><w:pBdr><w:bottom w:val=\"single\" w:sz=\"6\" w:space=\"1\" w:color=\"auto\"/></w:pBdr></w:pPr></w:p>".into(),
			"<w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>one</w:t></w:r></w:p>\
			 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"1\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>nested</w:t></w:r></w:p>\
			 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"1\"/></w:numPr></w:pPr><w:r><w:t>two</w:t></w:r></w:p>\
			 <w:p><w:pPr><w:numPr><w:ilvl w:val=\"0\"/><w:numId w:val=\"2\"/></w:numPr></w:pPr><w:r><w:t>first</w:t></w:r></w:p>".into(),
			paragraph("Zitat", &plain("quoted")),
			paragraph("IntenseQuote", &plain("intense")),
			paragraph("HTMLPreformatted", &run("<w:rStyle w:val=\"HTMLCode\"/>", "line one")),
			paragraph("HTMLPreformatted", &run("<w:rStyle w:val=\"HTMLCode\"/>", "line two")),
			paragraph("Beschriftung", &plain("Sales")),
			"<w:tbl><w:tblGrid><w:gridCol/><w:gridCol/></w:tblGrid>\
			 <w:tr><w:trPr><w:tblHeader/></w:trPr><w:tc><w:p><w:r><w:t>Name</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>Cost</w:t></w:r></w:p></w:tc></w:tr>\
			 <w:tr><w:tc><w:tcPr><w:gridSpan w:val=\"2\"/><w:shd w:val=\"clear\" w:color=\"auto\" w:fill=\"00FF00\"/></w:tcPr><w:p><w:r><w:t>wide</w:t></w:r></w:p></w:tc></w:tr>\
			 <w:tr><w:tc><w:tcPr><w:vMerge w:val=\"restart\"/></w:tcPr><w:p><w:r><w:t>tall</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>a</w:t></w:r></w:p></w:tc></w:tr>\
			 <w:tr><w:tc><w:tcPr><w:vMerge/></w:tcPr><w:p/></w:tc><w:tc><w:p><w:r><w:t>b</w:t></w:r></w:p></w:tc></w:tr>\
			 </w:tbl>".into(),
			paragraph(
				"",
				"<w:r><w:drawing><wp:inline><wp:extent cx=\"381000\" cy=\"190500\"/><wp:docPr id=\"1\" name=\"Picture 1\" descr=\"chart\"/>\
				 <a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\"><pic:pic>\
				 <pic:blipFill><a:blip r:embed=\"rId2\"/></pic:blipFill></pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r>",
			),
			"<w:p><w:r><mc:AlternateContent><mc:Choice Requires=\"wps\"><w:drawing><wp:anchor><wp:extent cx=\"100\" cy=\"100\"/>\
			 <a:graphic><a:graphicData uri=\"http://schemas.microsoft.com/office/word/2010/wordprocessingShape\"><wps:wsp><wps:txbx>\
			 <w:txbxContent><w:p><w:r><w:t>boxed</w:t></w:r></w:p></w:txbxContent></wps:txbx></wps:wsp></a:graphicData></a:graphic>\
			 </wp:anchor></w:drawing></mc:Choice><mc:Fallback><w:pict><v:shape><v:textbox><w:txbxContent><w:p><w:r><w:t>fallback</w:t></w:r></w:p>\
			 </w:txbxContent></v:textbox></v:shape></w:pict></mc:Fallback></mc:AlternateContent></w:r></w:p>".into(),
			paragraph(
				"",
				&[
					plain("noted"),
					"<w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:footnoteReference w:id=\"1\"/></w:r>".into(),
					plain(" and ended"),
					"<w:r><w:rPr><w:rStyle w:val=\"EndnoteReference\"/></w:rPr><w:endnoteReference w:id=\"1\"/></w:r>".into(),
				]
				.concat(),
			),
		]
		.concat();
		WordFixture::new(&body)
			.with_link("rId1", "https://example.com/site")
			.with_image("rId2", "image/png", super::super::office_picture::test::png())
			.with_styles(
				"<w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault></w:docDefaults>\
				 <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Standard\"><w:name w:val=\"Normal\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"Title\"><w:name w:val=\"Title\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"berschrift2\"><w:name w:val=\"heading 2\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"Zitat\"><w:name w:val=\"Quote\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"IntenseQuote\"><w:name w:val=\"Intense Quote\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"HTMLPreformatted\"><w:name w:val=\"HTML Preformatted\"/></w:style>\
				 <w:style w:type=\"character\" w:styleId=\"HTMLCode\"><w:name w:val=\"HTML Code\"/></w:style>\
				 <w:style w:type=\"paragraph\" w:styleId=\"Beschriftung\"><w:name w:val=\"caption\"/></w:style>",
			)
			.with_numbering(
				"<w:abstractNum w:abstractNumId=\"0\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"bullet\"/></w:lvl><w:lvl w:ilvl=\"1\"><w:numFmt w:val=\"bullet\"/></w:lvl></w:abstractNum>\
				 <w:abstractNum w:abstractNumId=\"1\"><w:lvl w:ilvl=\"0\"><w:numFmt w:val=\"decimal\"/></w:lvl></w:abstractNum>\
				 <w:num w:numId=\"1\"><w:abstractNumId w:val=\"0\"/></w:num><w:num w:numId=\"2\"><w:abstractNumId w:val=\"1\"/></w:num>",
			)
			.with_footnote(
				"<w:p><w:r><w:rPr><w:rStyle w:val=\"FootnoteReference\"/></w:rPr><w:footnoteRef/></w:r><w:r><w:t xml:space=\"preserve\"> the note</w:t></w:r></w:p>",
			)
			.with_endnote("<w:p><w:r><w:endnoteRef/></w:r><w:r><w:t xml:space=\"preserve\"> the end</w:t></w:r></w:p>")
			.with_header(&paragraph("", &plain("Header text")))
			.with_footer(&paragraph("", &plain("Footer text")))
			.with_properties("<dc:title>Covering</dc:title>")
			.bytes()
	}

	#[beet_core::test]
	fn reads_a_word_file_as_html() {
		let (mut world, root) = parse(&covering()).unwrap();
		world
			.entity(root)
			.get::<DocMeta>()
			.unwrap()
			.title
			.clone()
			.xpect_eq(Some("Covering".into()));
		html(&mut world, root)
			// the default header first, a bookmark the block's id, `_GoBack` dropped
			.xpect_starts_with("<header><p>Header text</p></header><h1 id=\"cover\">Cover</h1><h2>Looks</h2>")
			.xpect_contains("<p>plain <strong>bold</strong><em>italic</em><u>under</u><s>struckdouble</s><sup>up</sup><sub>down</sub><span class=")
			.xpect_contains("<mark>marked</mark><mark class=")
			.xpect_contains("<code>code</code><span class=")
			// a run's look nests outermost first, every other glyph its text
			.xpect_contains("><strong><em>all</em></strong></span>a b\u{3A9}\u{2011}<br />after</p>")
			.xpect_contains("<p class=")
			.xpect_contains("<p><a href=\"https://example.com/site\">site</a><a href=\"#cover\">back</a><a href=\"https://example.com/field\">field</a>3</p>")
			.xpect_contains("<p><input type=\"checkbox\" checked /> Name</p>")
			.xpect_contains("<p><ins datetime=\"2024-03-01T00:00:00Z\">added</ins><del datetime=\"2024-03-02T00:00:00Z\">gone</del></p>")
			.xpect_contains("<p>before</p><div class=\"page-break\"></div><p>after</p><hr />")
			.xpect_contains("<ul><li>one<ul><li>nested</li></ul></li><li>two</li></ul><ol><li>first</li></ol>")
			.xpect_contains("<blockquote><p>quoted</p><p>intense</p></blockquote><pre><code>line one\nline two</code></pre>")
			.xpect_contains("<table><caption>Sales</caption><thead><tr><th>Name</th><th>Cost</th></tr></thead><tbody><tr><td colspan=\"2\" class=")
			.xpect_contains("<tr><td rowspan=\"2\">tall</td><td>a</td></tr><tr><td>b</td></tr></tbody></table>")
			.xpect_contains("<p><img src=\"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAQAAAACCAYAAAA=\" alt=\"chart\" width=\"40\" height=\"20\" /></p>")
			// a text box's choice read, its fallback not
			.xpect_contains("<aside><p>boxed</p></aside>")
			.xpect_contains("<p>noted<sup><a href=\"#fn-1\">1</a></sup> and ended<sup><a href=\"#en-1\">1</a></sup></p>")
			.xpect_contains("<div id=\"fn-1\" class=\"footnote-definition\"><p>the note</p></div><div id=\"en-1\" class=\"footnote-definition\"><p>the end</p></div>")
			.xpect_ends_with("<footer><p>Footer text</p></footer>")
			.xnot()
			.xpect_contains("fallback");
	}

	/// Each direct look resolves through the cascade: a colour, a fill, a
	/// size against the document's default, a highlight other than yellow,
	/// an explicit off, an alignment, a theme slot as its token.
	#[beet_core::test]
	fn reads_direct_looks() {
		let bytes = WordFixture::new(&[
			paragraph("", &run("<w:color w:val=\"FF0000\"/>", "red")),
			paragraph("", &run("<w:shd w:val=\"clear\" w:fill=\"00FFFF\"/>", "filled")),
			paragraph("", &run("<w:sz w:val=\"28\"/>", "big")),
			paragraph("", &run("<w:sz w:val=\"22\"/>", "default")),
			paragraph("", &run("<w:highlight w:val=\"green\"/>", "green")),
			paragraph("Heading1", &run("<w:b w:val=\"0\"/>", "light")),
			format!("<w:p><w:pPr><w:jc w:val=\"right\"/></w:pPr>{}</w:p>", run("", "right")),
			paragraph("", &run("<w:color w:val=\"4472C4\" w:themeColor=\"accent1\" w:themeShade=\"80\"/>", "shaded")),
		]
		.concat())
		.with_styles("<w:docDefaults><w:rPrDefault><w:rPr><w:sz w:val=\"22\"/></w:rPr></w:rPrDefault></w:docDefaults>")
		.bytes();
		let (mut world, _) = parse(&bytes).unwrap();
		let block = |world: &mut World, words: &str| {
			let text = world
				.query::<(Entity, &Value)>()
				.iter(world)
				.find(|(_, value)| value.as_str().ok() == Some(words))
				.map(|(entity, _)| entity)
				.unwrap();
			world.entity(text).get::<ChildOf>().unwrap().parent()
		};
		let words = [
			"red", "filled", "big", "default", "green", "light", "right",
			"shaded",
		];
		let entities = words.map(|words| block(&mut world, words));
		world.with_state::<CascadeQuery, _>(|cascade| {
			let memo =
				&mut CascadeMemo::for_media(MediaContext::new(Medium::Print));
			let visual = entities
				.map(|entity| cascade.resolve(entity, memo).unwrap().visual);
			visual[0].foreground.xpect_eq(Some(Color::srgb(1., 0., 0.)));
			visual[1].background.xpect_eq(Some(Color::srgb(0., 1., 1.)));
			visual[2]
				.font_size
				.xpect_eq(Length::Px(OfficeUnits::px_from_pt(14.)));
			// the default size is no look of its own
			visual[3].font_size.xpect_eq(Length::Rem(1.));
			visual[4].background.xpect_eq(Some(Color::srgb(0., 1., 0.)));
			visual[5].font_weight.is_bold().xpect_false();
			visual[6].text_align.xpect_eq(TextAlign::Right);
			// a shaded slot computes, since no token names it
			visual[7]
				.foreground
				.map(OfficeColor::hex)
				.xpect_eq(Some("203965".to_string()));
		});
	}
}
