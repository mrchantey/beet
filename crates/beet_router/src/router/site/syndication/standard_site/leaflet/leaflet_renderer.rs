//! The Leaflet renderer: a page's tree as `pub.leaflet.content`.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// The Leaflet renderer: a page's tree as [`LeafletContent`], a
/// [`LexiconRenderer`] of `pub.leaflet.content`. An image is a blob when the
/// media resolve step fetched it, which a request naming a media ingest policy
/// asks for, and a link card otherwise.
///
/// Renders the body alone, never the whole `site.standard.document`: the
/// record needs what a page cannot know (the publication's at-uri, the
/// resolved contributors, the announcement), so a publish step assembles it.
/// A render is pure over the resolved tree, so a dry run renders exactly what
/// a publish writes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LeafletRenderer;

impl LexiconRenderer for LeafletRenderer {
	fn nsid(&self) -> Nsid { LeafletContent::NSID }

	fn render(&mut self, cx: &mut RenderContext) -> Result<OpenUnion> {
		let (request, root) = (cx.request, cx.entity);
		let content = cx.world.with_state::<LeafletQuery, _>(|query| {
			query.content(request, root)
		})?;
		OpenUnion::from_serde(LeafletContent::NSID, &content)
	}
}

impl Installer for LeafletRenderer {
	fn install(self, app: &mut App) {
		LexiconRenderers::register_on(app, self);
	}
}

/// What the Leaflet walk reads off a rendered tree.
#[derive(SystemParam)]
struct LeafletQuery<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	values: Query<'w, 's, &'static Value, Without<Element>>,
	tree: RenderTreeQuery<'w, 's>,
	attributes: AttributeQuery<'w, 's>,
	inline_blobs: Query<'w, 's, &'static InlineBlob>,
	metas: Query<'w, 's, &'static DocMeta>,
	unknowns: Query<'w, 's, &'static UnknownLexicon>,
	package: Option<Res<'w, PackageConfig>>,
	unregistered: UnregisteredQuery<'w, 's>,
	#[cfg(feature = "mermaid")]
	diagrams: Query<'w, 's, &'static MermaidDiagram>,
}

/// The tags a block walk steps into, whose own boxes Leaflet has no block for.
const CONTAINERS: &[&str] = &[
	"html",
	"body",
	"main",
	"article",
	"section",
	"div",
	"header",
	"footer",
	"aside",
	"nav",
	"figure",
	"figcaption",
	"details",
	"summary",
	"dl",
	"dd",
	"dt",
	"address",
	"form",
	"fieldset",
	"hgroup",
	"search",
	"dialog",
];

/// The inline tags whose text flows into the line with no facet of its own.
const INLINE: &[&str] = &[
	"span", "sup", "sub", "small", "abbr", "cite", "kbd", "samp", "var",
	"time", "q", "dfn", "bdi", "bdo", "wbr", "label",
];

impl LeafletQuery<'_, '_> {
	/// The document the tree at `root` renders as, answering `request`: the
	/// companion video, then the blocks.
	fn content(
		&self,
		request: &RequestParts,
		root: Entity,
	) -> Result<LeafletContent> {
		let mut blocks = Vec::new();
		if let Some(video) = self
			.metas
			.get(root)
			.ok()
			.and_then(|meta| meta.video_url.clone())
		{
			blocks.push(Self::video(&video)?);
		}
		self.blocks(request, root, &mut blocks)?;
		LeafletContent {
			pages: vec![LeafletPage {
				id: None,
				blocks: blocks
					.into_iter()
					.map(|block| BlockSlot { block })
					.collect(),
			}],
		}
		.xok()
	}

	/// The companion video's block: a YouTube video's embed, else a link card.
	fn video(url: &Url) -> Result<LeafletBlock> {
		match DocMeta::youtube_id(url) {
			Some(id) => LeafletBlock::Iframe(IframeBlock {
				url: Uri::parse(&format!(
					"https://www.youtube.com/embed/{id}"
				))?,
				aspect_ratio: Some(AspectRatio {
					width: 16,
					height: 9,
				}),
			}),
			None => LeafletBlock::Website(WebsiteBlock {
				src: Uri::parse(&url.to_string())?,
				title: None,
			}),
		}
		.xok()
	}

	/// The url the page `request` asked for answers at, which every relative
	/// link resolves against: the site's homepage over the request's path.
	///
	/// # Errors
	/// Errors naming `PackageConfig.homepage` when it is unset, since a
	/// record is read far from the site and a relative link there is broken.
	fn page_url(&self, request: &RequestParts) -> Result<Url> {
		let path = request.path_string();
		match self.package.as_deref() {
			Some(package) => package.absolute_url(&path),
			// no package declared is no homepage declared
			None => PackageConfig::default().absolute_url(&path),
		}
	}

	/// `href` as a link a reader anywhere follows: an absolute uri verbatim,
	/// a relative one resolved against the page's url.
	fn absolute(&self, request: &RequestParts, href: &str) -> Result<Uri> {
		match Uri::parse(href) {
			Ok(uri) => uri.xok(),
			Err(_) => Uri::parse(
				&self.page_url(request)?.join(Url::parse(href)?).to_string(),
			),
		}
	}

	/// The blocks the tree at `entity` writes, appended to `out`: block
	/// elements as their blocks, runs of inline content as text blocks.
	fn blocks(
		&self,
		request: &RequestParts,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
	) -> Result {
		let mut run = InlineRun::default();
		self.block_node(request, self.tree.resolve(entity), out, &mut run)?;
		run.flush_text(out);
		Ok(())
	}

	/// One node of a block walk answering `request`: a block element flushes the
	/// pending inline `run` and writes its block, inline content extends it.
	fn block_node(
		&self,
		request: &RequestParts,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(value) = self.values.get(entity) {
			run.push_text(&value.to_string());
			return Ok(());
		}
		// a member a reader kept whole goes back in its slot unchanged
		if let Ok(unknown) = self.unknowns.get(entity) {
			run.flush_text(out);
			out.push(LeafletBlock::Unknown(unknown.0.clone()));
			return Ok(());
		}
		let Ok(element) = self.elements.get(entity) else {
			return self.block_children(request, entity, out, run);
		};
		let tag = element.tag().to_ascii_lowercase();
		match tag.as_str() {
			// an embed is textless, but maps to a link card before the skip
			"iframe" => {
				run.flush_text(out);
				if let Some(block) = self.iframe(request, entity)? {
					out.push(block);
				}
			}
			tag if RenderTreeQuery::is_textless(tag) => {}
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				run.flush_text(out);
				let (plaintext, facets) = self.inline_text(request, entity)?;
				out.push(LeafletBlock::Header(HeaderBlock {
					level: tag[1..].parse().unwrap_or(1),
					plaintext,
					facets,
				}));
			}
			"p" => {
				run.flush_text(out);
				self.inline_children(request, entity, out, run)?;
				run.flush_text(out);
			}
			"blockquote" => {
				run.flush_text(out);
				out.push(self.blockquote(request, entity)?);
			}
			"pre" => {
				run.flush_text(out);
				out.push(LeafletBlock::Code(CodeBlock {
					plaintext: self
						.text(entity)
						.trim_end_matches('\n')
						.to_string(),
					language: self.code_language(entity),
				}));
			}
			"hr" => {
				run.flush_text(out);
				out.push(LeafletBlock::HorizontalRule(default()));
			}
			"ul" => {
				run.flush_text(out);
				out.push(LeafletBlock::UnorderedList(
					self.unordered_list(request, entity)?,
				));
			}
			"ol" => {
				run.flush_text(out);
				out.push(LeafletBlock::OrderedList(
					self.ordered_list(request, entity)?,
				));
			}
			"img" => {
				run.flush_text(out);
				out.push(self.image(request, entity)?);
			}
			"table" => {
				run.flush_text(out);
				out.push(LeafletBlock::Code(CodeBlock {
					plaintext: self.pipe_table(entity),
					language: None,
				}));
			}
			"figure" if self.is_diagram(entity) => {
				run.flush_text(out);
				out.push(self.diagram(entity));
			}
			"br" => run.push_break(),
			tag if CONTAINERS.contains(&tag) => {
				run.flush_text(out);
				self.block_children(request, entity, out, run)?;
				run.flush_text(out);
			}
			_ if self.is_inline(entity, &tag) => {
				self.inline_node(request, entity, out, run)?;
			}
			// an island or any other element nothing maps: its prose alone
			_ => {
				run.flush_text(out);
				let text = self.text(entity);
				if !text.trim().is_empty() {
					out.push(LeafletBlock::Text(TextBlock {
						plaintext: collapse_whitespace(&text),
						facets: Vec::new(),
					}));
				}
			}
		}
		Ok(())
	}

	fn block_children(
		&self,
		request: &RequestParts,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		for child in self.tree.children(entity) {
			self.block_node(request, child, out, run)?;
		}
		Ok(())
	}

	/// Whether the element at `entity` is markup flowing within a line rather
	/// than a block of its own.
	fn is_inline(&self, entity: Entity, tag: &str) -> bool {
		!self.unregistered.contains(entity)
			&& (InlineRun::feature(tag).is_some()
				|| tag == "a"
				|| INLINE.contains(&tag))
	}

	fn inline_children(
		&self,
		request: &RequestParts,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		for child in self.tree.children(entity) {
			self.inline_node(request, child, out, run)?;
		}
		Ok(())
	}

	/// One node of an inline run: text extends it, a marked element opens a
	/// facet over what it contains, and an image splits the run around its
	/// own block.
	fn inline_node(
		&self,
		request: &RequestParts,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(value) = self.values.get(entity) {
			run.push_text(&value.to_string());
			return Ok(());
		}
		let Ok(element) = self.elements.get(entity) else {
			return self.inline_children(request, entity, out, run);
		};
		let tag = element.tag().to_ascii_lowercase();
		match tag.as_str() {
			tag if RenderTreeQuery::is_textless(tag) => Ok(()),
			"br" => {
				run.push_break();
				Ok(())
			}
			"img" => {
				run.split(out);
				out.push(self.image(request, entity)?);
				Ok(())
			}
			"a" => {
				let feature = self
					.attribute(entity, "href")
					.map(|href| self.absolute(request, &href))
					.transpose()?
					.map(|uri| LeafletFacetFeature::Link { uri });
				run.open(feature);
				self.inline_children(request, entity, out, run)?;
				run.close();
				Ok(())
			}
			tag => {
				run.open(InlineRun::feature(tag));
				self.inline_children(request, entity, out, run)?;
				run.close();
				Ok(())
			}
		}
	}

	/// The inline text and facets of the element at `entity`, for a block
	/// that holds one run (a heading, a list item).
	fn inline_text(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<(String, Vec<LeafletFacet>)> {
		let mut run = InlineRun::default();
		let mut out = Vec::new();
		self.inline_children(request, entity, &mut out, &mut run)?;
		run.trim_end();
		(run.plaintext, run.facets).xok()
	}

	/// A quotation: its paragraphs joined by a blank line, their facets
	/// shifted along with them.
	fn blockquote(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<LeafletBlock> {
		let mut inner = Vec::new();
		let mut run = InlineRun::default();
		self.block_children(request, entity, &mut inner, &mut run)?;
		run.flush_text(&mut inner);
		let mut quote = BlockquoteBlock::default();
		for block in inner {
			let (plaintext, facets) = match block {
				LeafletBlock::Text(text) => (text.plaintext, text.facets),
				LeafletBlock::Header(header) => {
					(header.plaintext, header.facets)
				}
				LeafletBlock::Code(code) => (code.plaintext, Vec::new()),
				_ => continue,
			};
			if !quote.plaintext.is_empty() {
				quote.plaintext.push_str("\n\n");
			}
			let shift = quote.plaintext.len();
			quote.plaintext.push_str(&plaintext);
			quote.facets.extend(facets.into_iter().map(|mut facet| {
				facet.index.byte_start += shift;
				facet.index.byte_end += shift;
				facet
			}));
		}
		LeafletBlock::Blockquote(quote).xok()
	}

	/// The bulleted list at `entity`.
	fn unordered_list(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<UnorderedListBlock> {
		let mut list = UnorderedListBlock::default();
		for item in self.child_elements(entity, "li") {
			let (content, nested) = self.list_item(request, item)?;
			let mut item = UnorderedListItem {
				content,
				..default()
			};
			for nested in nested {
				match nested {
					NestedList::Unordered(nested) => {
						item.children.extend(nested.children)
					}
					NestedList::Ordered(nested) => {
						item.ordered_list_children = Some(nested)
					}
				}
			}
			list.children.push(item);
		}
		list.xok()
	}

	/// The numbered list at `entity`.
	fn ordered_list(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<OrderedListBlock> {
		let mut list = OrderedListBlock {
			start_index: self
				.attribute(entity, "start")
				.and_then(|start| start.parse().ok()),
			..default()
		};
		for item in self.child_elements(entity, "li") {
			let (content, nested) = self.list_item(request, item)?;
			let mut item = OrderedListItem {
				content,
				..default()
			};
			for nested in nested {
				match nested {
					NestedList::Ordered(nested) => {
						item.children.extend(nested.children)
					}
					NestedList::Unordered(nested) => {
						item.unordered_list_children = Some(nested)
					}
				}
			}
			list.children.push(item);
		}
		list.xok()
	}

	/// A list item's own text and the lists nested in it.
	fn list_item(
		&self,
		request: &RequestParts,
		item: Entity,
	) -> Result<(TextBlock, Vec<NestedList>)> {
		let mut run = InlineRun::default();
		let mut out = Vec::new();
		let mut nested = Vec::new();
		for child in self.tree.children(item) {
			match self
				.elements
				.get(child)
				.map(|element| element.tag().to_ascii_lowercase())
				.as_deref()
			{
				Ok("ul") => nested.push(NestedList::Unordered(
					self.unordered_list(request, child)?,
				)),
				Ok("ol") => nested.push(NestedList::Ordered(
					self.ordered_list(request, child)?,
				)),
				// a loose list wraps each item's text in a paragraph
				Ok("p") => {
					run.push_text(" ");
					self.inline_children(request, child, &mut out, &mut run)?;
				}
				_ => self.inline_node(request, child, &mut out, &mut run)?,
			}
		}
		run.trim_end();
		(
			TextBlock {
				plaintext: run.plaintext,
				facets: run.facets,
			},
			nested,
		)
			.xok()
	}

	/// An image: its fetched blob, else a link card to it.
	fn image(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<LeafletBlock> {
		let alt = self.attribute(entity, "alt").filter(|alt| !alt.is_empty());
		let source = self
			.attributes
			.all(entity)
			.into_iter()
			.find(|(_, key, _)| key.as_str() == "src");
		let blob = source.and_then(|(attribute, _, _)| {
			self.inline_blobs.get(attribute).ok()
		});
		if let Some(blob) = blob {
			match imagesize::blob_size(&blob.bytes) {
				Ok(size) => {
					return LeafletBlock::Image(ImageBlock {
						image: blob.blob_ref(),
						alt,
						aspect_ratio: AspectRatio {
							width: size.width as u32,
							height: size.height as u32,
						},
						width: None,
					})
					.xok();
				}
				// a format with no readable size cannot be an image block
				Err(err) => warn!(
					"leaflet: `{}` has no size a header gives ({err}), \
					 linking it instead",
					blob.link
				),
			}
		}
		let src = match blob {
			Some(blob) => Uri::parse(&blob.link.to_string())?,
			None => self.absolute(
				request,
				&source
					.and_then(|(_, _, value)| {
						value.as_str().ok().map(str::to_string)
					})
					.ok_or_else(|| bevyhow!("an `img` with no `src`"))?,
			)?,
		};
		LeafletBlock::Website(WebsiteBlock { src, title: alt }).xok()
	}

	/// An embedded page as a link card, the url a human shares (the watch
	/// url a YouTube embed carries as `alt-src`) where it has one.
	fn iframe(
		&self,
		request: &RequestParts,
		entity: Entity,
	) -> Result<Option<LeafletBlock>> {
		let Some(src) = self
			.attribute(entity, "alt-src")
			.or_else(|| self.attribute(entity, "src"))
		else {
			return Ok(None);
		};
		LeafletBlock::Website(WebsiteBlock {
			src: self.absolute(request, &src)?,
			title: self.attribute(entity, "title"),
		})
		.xmap(Some)
		.xok()
	}

	/// The language a `pre`'s code names in its class.
	fn code_language(&self, pre: Entity) -> Option<String> {
		self.child_elements(pre, "code")
			.into_iter()
			.chain([pre])
			.find_map(|entity| self.attribute(entity, "class"))
			.and_then(|class| {
				class
					.split_whitespace()
					.map(|class| {
						class.strip_prefix("language-").unwrap_or(class)
					})
					.find(|class| !class.is_empty())
					.map(str::to_string)
			})
	}

	/// Whether the figure at `entity` is a mermaid diagram.
	fn is_diagram(&self, entity: Entity) -> bool {
		#[cfg(feature = "mermaid")]
		if self.diagrams.contains(entity) {
			return true;
		}
		self.attribute(entity, "class").is_some_and(|class| {
			class.split_whitespace().any(|class| class == "diagram")
		})
	}

	/// A mermaid diagram as the fence it was written as.
	fn diagram(&self, entity: Entity) -> LeafletBlock {
		#[cfg(feature = "mermaid")]
		if let Ok(diagram) = self.diagrams.get(entity) {
			return LeafletBlock::Code(CodeBlock {
				plaintext: diagram.source.trim_end().to_string(),
				language: Some("mermaid".into()),
			});
		}
		LeafletBlock::Code(CodeBlock {
			plaintext: self.text(entity).trim_end().to_string(),
			language: Some("mermaid".into()),
		})
	}

	/// The table at `entity` as a pipe table, its columns padded to align.
	fn pipe_table(&self, table: Entity) -> String {
		let mut rows: Vec<Vec<String>> = Vec::new();
		let mut header_rows = 0;
		for row in self
			.tree
			.iter_descendants_inclusive_depth_first(table)
			.filter(|entity| self.is_tag(*entity, "tr"))
		{
			let cells = self
				.tree
				.children(row)
				.filter(|cell| {
					self.is_tag(*cell, "td") || self.is_tag(*cell, "th")
				})
				.map(|cell| collapse_whitespace(&self.text(cell)))
				.collect();
			let is_header = !self.child_elements(row, "th").is_empty();
			if is_header && rows.len() == header_rows {
				header_rows += 1;
			}
			rows.push(cells);
		}
		let columns = rows.iter().map(Vec::len).max().unwrap_or_default();
		let widths = (0..columns)
			.map(|column| {
				rows.iter()
					.filter_map(|row| row.get(column))
					.map(|cell| cell.chars().count())
					.max()
					.unwrap_or_default()
					.max(3)
			})
			.collect::<Vec<_>>();
		let line = |cells: Vec<String>| {
			let cells = widths
				.iter()
				.enumerate()
				.map(|(column, width)| {
					let cell = cells.get(column).cloned().unwrap_or_default();
					let pad = width.saturating_sub(cell.chars().count());
					format!("{cell}{}", " ".repeat(pad))
				})
				.collect::<Vec<_>>();
			format!("| {} |", cells.join(" | "))
		};
		let mut out = Vec::new();
		for (index, row) in rows.into_iter().enumerate() {
			out.push(line(row));
			if index + 1 == header_rows.max(1) {
				out.push(line(
					widths.iter().map(|width| "-".repeat(*width)).collect(),
				));
			}
		}
		out.join("\n")
	}

	/// The value of `key` on the element at `entity`, as text.
	fn attribute(&self, entity: Entity, key: &str) -> Option<String> {
		self.attributes
			.all(entity)
			.into_iter()
			.find(|(_, attribute, _)| attribute.as_str() == key)
			.and_then(|(_, _, value)| value.as_str().ok().map(str::to_string))
	}

	/// Whether `entity` is an element tagged `tag`.
	fn is_tag(&self, entity: Entity, tag: &str) -> bool {
		self.elements
			.get(entity)
			.is_ok_and(|element| element.tag().eq_ignore_ascii_case(tag))
	}

	/// The child elements of `entity` tagged `tag`.
	fn child_elements(&self, entity: Entity, tag: &str) -> Vec<Entity> {
		self.tree
			.children(entity)
			.filter(|child| self.is_tag(*child, tag))
			.collect()
	}

	/// Every text node under `entity` verbatim, in document order.
	fn text(&self, entity: Entity) -> String {
		self.tree
			.iter_descendants_inclusive_depth_first(entity)
			.filter_map(|entity| self.values.get(entity).ok())
			.map(|value| value.to_string())
			.collect()
	}
}

/// A list nested inside a list item.
enum NestedList {
	Unordered(UnorderedListBlock),
	Ordered(OrderedListBlock),
}

/// A run of inline content being written into one text block: its plaintext,
/// its finished facets, and the facets still open.
#[derive(Default)]
struct InlineRun {
	plaintext: String,
	facets: Vec<LeafletFacet>,
	/// The open marks, each its feature (none for an element that only
	/// flows) and the byte it started at.
	open: Vec<(Option<LeafletFacetFeature>, usize)>,
}

impl InlineRun {
	/// The facet feature an inline `tag` marks its content with.
	fn feature(tag: &str) -> Option<LeafletFacetFeature> {
		match tag {
			"strong" | "b" => Some(LeafletFacetFeature::Bold),
			"em" | "i" => Some(LeafletFacetFeature::Italic),
			"code" => Some(LeafletFacetFeature::Code),
			"mark" => Some(LeafletFacetFeature::Highlight),
			"u" | "ins" => Some(LeafletFacetFeature::Underline),
			"s" | "del" | "strike" => Some(LeafletFacetFeature::Strikethrough),
			_ => None,
		}
	}

	/// Append `text`, collapsing whitespace as a browser lays prose out: a
	/// run never starts with or doubles a space.
	fn push_text(&mut self, text: &str) {
		for char in text.chars() {
			if char.is_whitespace() {
				if !self.plaintext.is_empty()
					&& !self.plaintext.ends_with([' ', '\n'])
				{
					self.plaintext.push(' ');
				}
			} else {
				self.plaintext.push(char);
			}
		}
	}

	/// A line break, a `<br>`.
	fn push_break(&mut self) {
		let len = self.plaintext.trim_end_matches(' ').len();
		self.plaintext.truncate(len);
		self.plaintext.push('\n');
	}

	/// Open a mark at the current position.
	fn open(&mut self, feature: Option<LeafletFacetFeature>) {
		self.open.push((feature, self.plaintext.len()));
	}

	/// Close the innermost mark, recording its facet when it covers anything.
	fn close(&mut self) {
		if let Some((Some(feature), start)) = self.open.pop() {
			self.push_facet(feature, start, self.plaintext.len());
		}
	}

	fn push_facet(
		&mut self,
		feature: LeafletFacetFeature,
		start: usize,
		end: usize,
	) {
		let end = end.min(self.plaintext.trim_end().len());
		if start < end {
			self.facets.push(LeafletFacet {
				index: LeafletByteSlice {
					byte_start: start,
					byte_end: end,
				},
				features: vec![feature],
			});
		}
	}

	/// Drop trailing whitespace, clamping the facets to it.
	fn trim_end(&mut self) {
		let len = self.plaintext.trim_end().len();
		self.plaintext.truncate(len);
		for facet in &mut self.facets {
			facet.index.byte_end = facet.index.byte_end.min(len);
		}
		self.facets
			.retain(|facet| facet.index.byte_start < facet.index.byte_end);
	}

	/// End this run as a text block when it holds any text, leaving it empty.
	fn flush_text(&mut self, out: &mut Vec<LeafletBlock>) {
		self.trim_end();
		if !self.plaintext.is_empty() {
			out.push(LeafletBlock::Text(TextBlock {
				plaintext: core::mem::take(&mut self.plaintext),
				facets: core::mem::take(&mut self.facets),
			}));
		}
		self.plaintext.clear();
		self.facets.clear();
	}

	/// End this run mid-markup, ie around an image: the open marks close
	/// here and reopen at the start of the run that follows.
	fn split(&mut self, out: &mut Vec<LeafletBlock>) {
		let end = self.plaintext.len();
		for (feature, start) in self.open.clone() {
			if let Some(feature) = feature {
				self.push_facet(feature, start, end);
			}
		}
		self.flush_text(out);
		for (_, start) in &mut self.open {
			*start = 0;
		}
	}
}

/// `text` with every run of whitespace one space, trimmed.
fn collapse_whitespace(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// A world rendering to Leaflet, its site at `homepage`.
	fn world_at(homepage: Option<&str>) -> World {
		let mut world = world_ext::ui_world();
		world.insert_resource(PackageConfig {
			homepage: homepage.map(|homepage| Url::parse(homepage).unwrap()),
			..default()
		});
		world
			.resource_mut::<LexiconRenderers>()
			.register(LeafletRenderer);
		world
	}

	/// A world rendering to Leaflet, its site at `https://beet.org`.
	fn world() -> World { world_at(Some("https://beet.org")) }

	/// The tree at `root` as Leaflet's content, answering a request for
	/// `/blog/post`.
	fn leaflet(world: &mut World, root: Entity) -> Result<OpenUnion> {
		LexiconRenderers::render(
			world,
			root,
			&RequestParts::get("/blog/post"),
			&LeafletContent::NSID,
		)
	}

	/// `member` as json.
	fn to_json(member: OpenUnion) -> serde_json::Value {
		serde_json::to_value(Value::from(member)).unwrap()
	}

	/// The blocks the tree at `root` renders as, as json.
	fn blocks(world: &mut World, root: Entity) -> serde_json::Value {
		let content = to_json(leaflet(world, root).unwrap());
		content["pages"][0]["blocks"]
			.as_array()
			.unwrap()
			.iter()
			.map(|slot| slot["block"].clone())
			.collect()
	}

	/// The blocks `bundle` renders as.
	fn render(bundle: impl Bundle) -> serde_json::Value {
		let mut world = world();
		let root = world.spawn(bundle).id();
		blocks(&mut world, root)
	}

	fn json(text: &str) -> serde_json::Value {
		serde_json::from_str(text).unwrap()
	}

	#[beet_core::test]
	fn maps_headings_and_paragraphs() {
		render(rsx! {
			<article>
				<h1>"Title"</h1>
				<p>"First paragraph."</p>
				<h3>"Sub"</h3>
				<p>"Second   paragraph,\n wrapped."</p>
			</article>
		})
		.xpect_eq(json(
			r#"[
				{"$type":"pub.leaflet.blocks.header","level":1,"plaintext":"Title"},
				{"$type":"pub.leaflet.blocks.text","plaintext":"First paragraph."},
				{"$type":"pub.leaflet.blocks.header","level":3,"plaintext":"Sub"},
				{"$type":"pub.leaflet.blocks.text","plaintext":"Second paragraph, wrapped."}
			]"#,
		));
	}

	/// Inline markup is facets over byte ranges, nested marks overlapping,
	/// offsets counted in UTF-8 bytes.
	#[beet_core::test]
	fn inline_markup_is_facets() {
		let blocks = render(rsx! {
			<p>"Grüße "<strong>"bold "<em>"both"</em></strong>" "<code>"code"</code>" "<a href="https://beet.org">"link"</a>"."</p>
		});
		let block = &blocks[0];
		block["plaintext"].xpect_eq(json(r#""Grüße bold both code link.""#));
		// `Grüße ` is 8 bytes, `ü` and `ß` being two each
		block["facets"].xpect_eq(json(
			r#"[
				{"index":{"byteStart":13,"byteEnd":17},"features":[{"$type":"pub.leaflet.richtext.facet#italic"}]},
				{"index":{"byteStart":8,"byteEnd":17},"features":[{"$type":"pub.leaflet.richtext.facet#bold"}]},
				{"index":{"byteStart":18,"byteEnd":22},"features":[{"$type":"pub.leaflet.richtext.facet#code"}]},
				{"index":{"byteStart":23,"byteEnd":27},"features":[{"$type":"pub.leaflet.richtext.facet#link","uri":"https://beet.org"}]}
			]"#,
		));
	}

	/// Every inline mark the format names.
	#[beet_core::test]
	fn maps_every_mark() {
		render(rsx! {
			<p><b>"b"</b><i>"i"</i><mark>"m"</mark><u>"u"</u><s>"s"</s><del>"d"</del></p>
		})[0]["facets"]
			.as_array()
			.unwrap()
			.iter()
			.map(|facet| {
				facet["features"][0]["$type"].as_str().unwrap().to_string()
			})
			.collect::<Vec<_>>()
			.xpect_eq(
				[
					"bold",
					"italic",
					"highlight",
					"underline",
					"strikethrough",
					"strikethrough",
				]
				.map(|feature| format!("pub.leaflet.richtext.facet#{feature}")),
			);
	}

	/// A relative link resolves against the homepage over the request's
	/// path, and with no homepage declared it is refused, naming the field.
	#[beet_core::test]
	fn resolves_relative_links() {
		let link = || rsx! { <p><a href="../docs">"docs"</a></p> };
		let mut world = world();
		let root = world.spawn(link()).id();
		blocks(&mut world, root)[0]["facets"][0]["features"][0]["uri"]
			.xpect_eq(json(r#""https://beet.org/docs""#));
		let mut world = world_at(None);
		let root = world.spawn(link()).id();
		leaflet(&mut world, root)
			.unwrap_err()
			.to_string()
			.xpect_contains("PackageConfig.homepage");
	}

	#[beet_core::test]
	fn joins_a_blockquote() {
		render(rsx! {
			<blockquote><p>"One "<em>"quote"</em></p><p>"Two"</p></blockquote>
		})
		.xpect_eq(json(
			r#"[{"$type":"pub.leaflet.blocks.blockquote","plaintext":"One quote\n\nTwo","facets":[{"index":{"byteStart":4,"byteEnd":9},"features":[{"$type":"pub.leaflet.richtext.facet#italic"}]}]}]"#,
		));
	}

	/// A code block keeps its text verbatim and names its language from the
	/// class, bare or prefixed.
	#[beet_core::test]
	fn maps_code_blocks() {
		render(rsx! {
			<div>
				<pre><code class="rust">"fn main() {\n    run();\n}"</code></pre>
				<pre><code class="language-js">"let x"</code></pre>
				<pre><code>"plain"</code></pre>
				<hr/>
			</div>
		})
		.xpect_eq(json(
			r#"[
				{"$type":"pub.leaflet.blocks.code","plaintext":"fn main() {\n    run();\n}","language":"rust"},
				{"$type":"pub.leaflet.blocks.code","plaintext":"let x","language":"js"},
				{"$type":"pub.leaflet.blocks.code","plaintext":"plain"},
				{"$type":"pub.leaflet.blocks.horizontalRule"}
			]"#,
		));
	}

	/// Lists nest through `children`, a list of the other kind through its
	/// own field, and a numbered list keeps its start.
	#[beet_core::test]
	fn maps_lists() {
		render(rsx! {
			<div>
				<ul>
					<li>"one"<ul><li>"one a"</li></ul></li>
					<li>"two"<ol><li>"two i"</li></ol></li>
				</ul>
				<ol start="3"><li><p>"three"</p></li></ol>
			</div>
		})
		.xpect_eq(json(
			r#"[
				{"$type":"pub.leaflet.blocks.unorderedList","children":[
					{"$type":"pub.leaflet.blocks.unorderedList#listItem","content":{"$type":"pub.leaflet.blocks.text","plaintext":"one"},
						"children":[{"$type":"pub.leaflet.blocks.unorderedList#listItem","content":{"$type":"pub.leaflet.blocks.text","plaintext":"one a"}}]},
					{"$type":"pub.leaflet.blocks.unorderedList#listItem","content":{"$type":"pub.leaflet.blocks.text","plaintext":"two"},
						"orderedListChildren":{"$type":"pub.leaflet.blocks.orderedList","children":[{"$type":"pub.leaflet.blocks.orderedList#listItem","content":{"$type":"pub.leaflet.blocks.text","plaintext":"two i"}}]}}
				]},
				{"$type":"pub.leaflet.blocks.orderedList","startIndex":3,"children":[
					{"$type":"pub.leaflet.blocks.orderedList#listItem","content":{"$type":"pub.leaflet.blocks.text","plaintext":"three"}}
				]}
			]"#,
		));
	}

	/// An image the media resolve step fetched is an image block, its size
	/// read off the header; one it did not is a link card to its absolute url,
	/// and an image inside a paragraph splits the text around it.
	#[beet_core::test]
	fn maps_images() {
		let mut world = world();
		let root = world
			.spawn(rsx! {
				<div>
					<p>"Before "<img src="./fetched.png" alt="Fetched"/>" after."</p>
					<img src="/linked.png" alt="Linked"/>
				</div>
			})
			.id();
		let fetched = world
			.query::<(Entity, &Attribute, &Value)>()
			.iter(&world)
			.find(|(_, key, value)| {
				key.as_str() == "src"
					&& value.as_str().ok() == Some("./fetched.png")
			})
			.map(|(entity, _, _)| entity)
			.unwrap();
		let blob = InlineBlob::new(
			Url::parse("https://beet.org/blog/fetched.png").unwrap(),
			MediaBytes::new(MediaType::Png, png(1448, 1074)),
		);
		world.entity_mut(fetched).insert(blob.clone());
		let blob_ref = serde_json::to_value(blob.blob_ref()).unwrap();
		blocks(&mut world, root).xpect_eq(json(&format!(
			r#"[
				{{"$type":"pub.leaflet.blocks.text","plaintext":"Before"}},
				{{"$type":"pub.leaflet.blocks.image","image":{blob_ref},"alt":"Fetched","aspectRatio":{{"width":1448,"height":1074}}}},
				{{"$type":"pub.leaflet.blocks.text","plaintext":"after."}},
				{{"$type":"pub.leaflet.blocks.website","src":"https://beet.org/linked.png","title":"Linked"}}
			]"#
		)));
	}

	/// A table is a pipe table in a code block, its columns aligned.
	#[beet_core::test]
	fn degrades_a_table() {
		render(rsx! {
			<div class="table-scroll"><table>
				<thead><tr><th>"Name"</th><th>"Kind"</th></tr></thead>
				<tbody><tr><td>"beet"</td><td>"engine"</td></tr><tr><td>"ecs"</td><td>"model"</td></tr></tbody>
			</table></div>
		})
		.xpect_eq(json(
			r#"[{"$type":"pub.leaflet.blocks.code","plaintext":"| Name | Kind   |\n| ---- | ------ |\n| beet | engine |\n| ecs  | model  |"}]"#,
		));
	}

	/// A mermaid fence is a code block in the language it was written in.
	#[beet_core::test]
	fn degrades_a_mermaid_fence() {
		render(rsx! { <pre><code class="mermaid">"graph LR; A --> B"</code></pre> })
			.xpect_eq(json(
				r#"[{"$type":"pub.leaflet.blocks.code","plaintext":"graph LR; A --> B","language":"mermaid"}]"#,
			));
	}

	/// A collected diagram reads back as its source.
	#[cfg(feature = "mermaid")]
	#[beet_core::test]
	fn degrades_a_collected_diagram() {
		render((
			Element::new("figure"),
			MermaidDiagram::new("graph LR; A --> B\n"),
			Attribute::bundle("class", "diagram"),
		))
		.xpect_eq(json(
			r#"[{"$type":"pub.leaflet.blocks.code","plaintext":"graph LR; A --> B","language":"mermaid"}]"#,
		));
	}

	/// An iframe is a link card to the url a human shares.
	#[beet_core::test]
	fn degrades_an_iframe() {
		render(rsx! {
			<iframe src="https://www.youtube.com/embed/7koepBSRoUI" alt-src="https://youtu.be/7koepBSRoUI" title="Talk"></iframe>
		})
		.xpect_eq(json(
			r#"[{"$type":"pub.leaflet.blocks.website","src":"https://youtu.be/7koepBSRoUI","title":"Talk"}]"#,
		));
	}

	/// An element nothing maps is dropped, its text kept.
	#[beet_core::test]
	fn degrades_an_unregistered_element() {
		render((
			Element::new("Counter"),
			Unregistered::new("Counter", Value::map()),
			children![Value::from("You clicked 3 times")],
		))
		.xpect_eq(json(
			r#"[{"$type":"pub.leaflet.blocks.text","plaintext":"You clicked 3 times"}]"#,
		));
		render(rsx! { <div><my-island>"  "</my-island></div> })
			.xpect_eq(json("[]"));
	}

	/// The companion video leads as Leaflet's own records embed one.
	#[beet_core::test]
	fn leads_with_the_companion_video() {
		render((
			DocMeta {
				video_url: Some(Url::parse("https://youtu.be/7koepBSRoUI").unwrap()),
				..default()
			},
			rsx! { <p>"Body"</p> },
		))[0]
			.xpect_eq(json(
				r#"{"$type":"pub.leaflet.blocks.iframe","url":"https://www.youtube.com/embed/7koepBSRoUI","aspectRatio":{"width":16,"height":9}}"#,
			));
	}

	/// The `--root=content` Leaflet render of the site's own blog post at
	/// `slug`, read off `site/routes/blog` and rendered as a request would
	/// render it, naming no media ingest policy so its media is linked and a
	/// run needs no assets.
	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	async fn blog_post(slug: &'static str) -> String {
		let mut world = syndication_world(Some("https://beet.org"));
		let blog = fs_ext::workspace_root().join("site/routes/blog");
		let router = world
			.spawn((
				BlobStore::new(FsStore::new(AbsPath::new(blog).unwrap())),
				Router::default(),
				children![RoutesDir::default()],
			))
			.flush();
		AsyncRunner::settle_async_tasks(&mut world).await;
		let content = world
			.run_async_then(async move |world| {
				LivePage::scoped(
					&world.entity(router),
					Request::get(format!("/{slug}"))
						.with_param("root", "content"),
					async |live| {
						live.render_lexicon(&LeafletContent::NSID).await
					},
				)
				.await
			})
			.await
			.unwrap();
		serde_json::to_string_pretty(&to_json(content)).unwrap()
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_full_stack_bevy() {
		blog_post("full-stack-bevy").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_ecs_router() {
		blog_post("ecs-router").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_bevys_five_and_beets_alive() {
		blog_post("bevys-five-and-beets-alive")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_action_time() {
		blog_post("action-time").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_declarative_state() {
		blog_post("declarative-state").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_folk_technology() {
		blog_post("folk-technology").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_malleable_application_framework() {
		blog_post("malleable-application-framework")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_its_all_been_done_before() {
		blog_post("its-all-been-done-before").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_multi_interface_applications() {
		blog_post("multi-interface-applications")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_application_level_homoiconicity() {
		blog_post("application-level-homoiconicity")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_user_modifiable_users() {
		blog_post("user-modifiable-users").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_gentle_slopes_up_lonely_mountains() {
		blog_post("gentle-slopes-up-lonely-mountains")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_engines_not_frameworks() {
		blog_post("engines-not-frameworks").await.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_atproto_isnt_malleable_yet() {
		blog_post("atproto-isnt-malleable-yet")
			.await
			.xpect_snapshot();
	}

	#[cfg(all(
		feature = "markdown_parser",
		feature = "native",
		not(target_arch = "wasm32")
	))]
	#[beet_core::test]
	async fn post_bevy_standardizes_malleable_software() {
		blog_post("bevy-standardizes-malleable-software")
			.await
			.xpect_snapshot();
	}
}
