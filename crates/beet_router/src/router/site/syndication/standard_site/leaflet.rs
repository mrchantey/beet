//! `pub.leaflet.content`, the one rich `content` format the atmosphere reads
//! today: Leaflet's own reader, and the third party standard.site readers built
//! against it.
//!
//! The format is a small block model, a subset of html. A beet post is a
//! markdown document whose parsed article is a superset of it, so the conversion
//! is lossy by design and the canonical page stays the full post. This is a
//! render target like html and charcell, and the only piece of another project's
//! lexicon beet learns.
//!
//! # The shape
//!
//! One `pages` entry per document, a `pub.leaflet.pages.linearDocument` holding
//! an array of `{block: <union member>}`. No canvas page: a post is linear.
//!
//! # What maps
//!
//! | parsed article node | block |
//! | --- | --- |
//! | `h1`..`h6` | `blocks.header`, `level` from the tag |
//! | `p` | `blocks.text` |
//! | `blockquote` | `blocks.blockquote`, its paragraphs joined by a blank line |
//! | `pre > code` | `blocks.code`, `language` from the code's class (`rust`, or `language-rust`) |
//! | `hr` | `blocks.horizontalRule` |
//! | `ul`, `ol` | `blocks.unorderedList`, `blocks.orderedList`; each item's `content` is a `blocks.text`, and a nested list rides `children` |
//! | `img` | `blocks.image`: the `BlobRef` of the `InlineBlob` the media resolve step left on its `src` attribute, and the `aspectRatio` the lexicon requires read from the bytes' header; under a `link` policy, `blocks.website` with the image's url |
//! | `a`, `strong`/`b`, `em`/`i`, `code` (inline), `mark`, `u`, `s`, `del` | not blocks: facets over the enclosing block's `plaintext` |
//!
//! # Facets
//!
//! A facet is a byte range over the block's `plaintext` plus a list of features,
//! so inline markup is a side table rather than a tree. Byte offsets into the
//! UTF-8 encoding, not chars and not graphemes. Features used: `#link`, `#bold`,
//! `#italic`, `#code`, `#highlight`, `#underline`, `#strikethrough`. Nesting is
//! expressed as two facets over overlapping ranges, which is the only way the
//! model can say it. Unused: `#didMention`, `#atMention`, `#id`, `#footnote`.
//!
//! # What degrades, and to what
//!
//! Every row here is a deliberate choice of the most legible lossy form, never a
//! silent drop:
//!
//! | node | degrade |
//! | --- | --- |
//! | `table` | a `blocks.code` with no language, holding the pipe table. Monospace keeps the columns aligned and a reader can see it is a table, where prose cannot say it at all |
//! | a mermaid fence | a `blocks.code` with `language: "mermaid"`, which is what the source said |
//! | `iframe` (the YouTube embed every video post carries) | a `blocks.website` with the watch url, ie a link card every reader can render, rather than `blocks.iframe`, which only Leaflet's own reader honours. The canonical page keeps the real embed |
//! | a client island or any other unregistered element | dropped, with its text content emitted as a `blocks.text` when it has any |
//!
//! Nothing beet authors maps to `blocks.math`, `blocks.poll`, `blocks.button`,
//! `blocks.drawing`, `blocks.embeddedCanvas`, `blocks.signup`,
//! `blocks.postsList`, `blocks.recommendedPubs`, `blocks.postHeader` or
//! `blocks.membersOnlyDelimiter`, so none is implemented.
//!
//! # The size ceiling
//!
//! A repo block is capped at a megabyte and a reader fetches a record whole, so
//! the content has a budget. The lexicon's escape hatch is `blobPages`, an
//! `application/json` blob holding the page array, and when it is set every blob
//! referenced inside it MUST be mirrored in the record's top level `blobs`
//! array, because a PDS scans only the top level of a current record when
//! deciding what to garbage collect.
//!
//! Beet does not write `blobPages`. The longest post in the corpus is about a
//! thousand words, which is single digit kilobytes of blocks, and images are
//! blobs rather than inline bytes. So the renderer asserts the serialised record
//! stays under the budget and fails loudly naming `blobPages` as the fix. An
//! unused code path that only a future post would exercise is a path nothing
//! tests.
//!
//! # The companion video
//!
//! A post's companion video is its `PageMeta::video_url`, which `ArticleHeader`
//! renders and a `--root=content` render therefore leaves out. The target
//! reads it off the `PageMeta` on the render root, as the cover image reads
//! `social_image_url`, and leads the document with it: a `blocks.iframe` of
//! the video's embed url at 16:9 for a YouTube video, the block Leaflet's own
//! records carry for one, else a `blocks.website` with the url.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// `pub.leaflet.content`: a post as Leaflet's block model, the object a
/// standard site document's `content` union carries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.content")]
pub struct LeafletContent {
	/// The document's pages, one linear page for a post.
	pub pages: Vec<LeafletPage>,
}

impl LeafletContent {
	/// The def a document's `content` names this format by.
	pub const NSID: Nsid = Nsid::new_static("pub.leaflet.content");
	/// The most a record's content may weigh: a repo block is capped at a
	/// megabyte.
	pub const BUDGET: usize = 1_000_000;
}

/// `pub.leaflet.pages.linearDocument`: one page, its blocks top to bottom.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.pages.linearDocument")]
pub struct LeafletPage {
	/// The page's id within its document.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub id: Option<String>,
	/// The blocks, each in its slot.
	pub blocks: Vec<BlockSlot>,
}

/// `pub.leaflet.pages.linearDocument#block`: one block in a page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.pages.linearDocument#block")]
pub struct BlockSlot {
	/// The block itself.
	pub block: LeafletBlock,
}

/// A member of the block union, each naming its own def in `$type`.
///
/// Read by that `$type`, since a struct's own serde tag is written but never
/// checked on read; a block this build does not know is refused naming it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum LeafletBlock {
	/// `pub.leaflet.blocks.text`
	Text(TextBlock),
	/// `pub.leaflet.blocks.header`
	Header(HeaderBlock),
	/// `pub.leaflet.blocks.blockquote`
	Blockquote(BlockquoteBlock),
	/// `pub.leaflet.blocks.code`
	Code(CodeBlock),
	/// `pub.leaflet.blocks.horizontalRule`
	HorizontalRule(HorizontalRuleBlock),
	/// `pub.leaflet.blocks.unorderedList`
	UnorderedList(UnorderedListBlock),
	/// `pub.leaflet.blocks.orderedList`
	OrderedList(OrderedListBlock),
	/// `pub.leaflet.blocks.image`
	Image(ImageBlock),
	/// `pub.leaflet.blocks.website`
	Website(WebsiteBlock),
	/// `pub.leaflet.blocks.iframe`
	Iframe(IframeBlock),
}

impl<'de> Deserialize<'de> for LeafletBlock {
	fn deserialize<D: serde::Deserializer<'de>>(
		deserializer: D,
	) -> Result<Self, D::Error> {
		use serde::de::Error;
		let value = Value::deserialize(deserializer)?;
		let r#type = value
			.as_map()
			.ok()
			.and_then(|map| map.get("$type").ok())
			.and_then(|r#type| r#type.as_str().ok())
			.map(SmolStr::new)
			.unwrap_or_default();
		match r#type.as_str() {
			"pub.leaflet.blocks.text" => value.into_serde().map(Self::Text),
			"pub.leaflet.blocks.header" => value.into_serde().map(Self::Header),
			"pub.leaflet.blocks.blockquote" => {
				value.into_serde().map(Self::Blockquote)
			}
			"pub.leaflet.blocks.code" => value.into_serde().map(Self::Code),
			"pub.leaflet.blocks.horizontalRule" => {
				value.into_serde().map(Self::HorizontalRule)
			}
			"pub.leaflet.blocks.unorderedList" => {
				value.into_serde().map(Self::UnorderedList)
			}
			"pub.leaflet.blocks.orderedList" => {
				value.into_serde().map(Self::OrderedList)
			}
			"pub.leaflet.blocks.image" => value.into_serde().map(Self::Image),
			"pub.leaflet.blocks.website" => {
				value.into_serde().map(Self::Website)
			}
			"pub.leaflet.blocks.iframe" => value.into_serde().map(Self::Iframe),
			other => Err(bevyhow!(
				"a Leaflet block this build does not write: `{other}`"
			)),
		}
		.map_err(D::Error::custom)
	}
}

/// `pub.leaflet.blocks.text`: a paragraph.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.text")]
pub struct TextBlock {
	/// The paragraph's text, no markup.
	pub plaintext: String,
	/// The inline markup over [`plaintext`](Self::plaintext).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub facets: Vec<Facet>,
}

/// `pub.leaflet.blocks.header`: a heading.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.header")]
pub struct HeaderBlock {
	/// The heading level, 1 to 6.
	pub level: u8,
	/// The heading's text.
	pub plaintext: String,
	/// The inline markup over [`plaintext`](Self::plaintext).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub facets: Vec<Facet>,
}

/// `pub.leaflet.blocks.blockquote`: a quotation.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.blockquote")]
pub struct BlockquoteBlock {
	/// The quotation's text, its paragraphs joined by a blank line.
	pub plaintext: String,
	/// The inline markup over [`plaintext`](Self::plaintext).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub facets: Vec<Facet>,
}

/// `pub.leaflet.blocks.code`: a code block, verbatim.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.code")]
pub struct CodeBlock {
	/// The code.
	pub plaintext: String,
	/// The language it is written in, ie `rust`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub language: Option<String>,
}

/// `pub.leaflet.blocks.horizontalRule`: a thematic break.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.horizontalRule")]
pub struct HorizontalRuleBlock {}

/// `pub.leaflet.blocks.unorderedList`: a bulleted list.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.unorderedList")]
pub struct UnorderedListBlock {
	/// The list's items.
	pub children: Vec<UnorderedListItem>,
}

/// `pub.leaflet.blocks.unorderedList#listItem`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.unorderedList#listItem")]
pub struct UnorderedListItem {
	/// The item's own text.
	pub content: TextBlock,
	/// A nested bulleted list.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub children: Vec<UnorderedListItem>,
	/// A nested numbered list.
	#[serde(
		default,
		rename = "orderedListChildren",
		skip_serializing_if = "Option::is_none"
	)]
	pub ordered_list_children: Option<OrderedListBlock>,
}

/// `pub.leaflet.blocks.orderedList`: a numbered list.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.orderedList")]
pub struct OrderedListBlock {
	/// The number the list starts at, 1 when absent.
	#[serde(
		default,
		rename = "startIndex",
		skip_serializing_if = "Option::is_none"
	)]
	pub start_index: Option<u32>,
	/// The list's items.
	pub children: Vec<OrderedListItem>,
}

/// `pub.leaflet.blocks.orderedList#listItem`
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.orderedList#listItem")]
pub struct OrderedListItem {
	/// The item's own text.
	pub content: TextBlock,
	/// A nested numbered list.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub children: Vec<OrderedListItem>,
	/// A nested bulleted list.
	#[serde(
		default,
		rename = "unorderedListChildren",
		skip_serializing_if = "Option::is_none"
	)]
	pub unordered_list_children: Option<UnorderedListBlock>,
}

/// `pub.leaflet.blocks.image`: an image, its bytes a blob.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.image")]
pub struct ImageBlock {
	/// The image's blob.
	pub image: BlobRef,
	/// The image's alt text.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub alt: Option<String>,
	/// The image's proportions, which the lexicon requires.
	#[serde(rename = "aspectRatio")]
	pub aspect_ratio: AspectRatio,
	/// The width to display the image at, in pixels, capped at the page's.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub width: Option<u32>,
}

/// `#aspectRatio`, a width to height proportion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectRatio {
	/// The width.
	pub width: u32,
	/// The height.
	pub height: u32,
}

/// `pub.leaflet.blocks.website`: a link card.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.website")]
pub struct WebsiteBlock {
	/// The page the card links.
	pub src: Uri,
	/// The card's title.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub title: Option<String>,
}

/// `pub.leaflet.blocks.iframe`: an embedded page.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.iframe")]
pub struct IframeBlock {
	/// The embedded page.
	pub url: Uri,
	/// The embed's proportions.
	#[serde(
		default,
		rename = "aspectRatio",
		skip_serializing_if = "Option::is_none"
	)]
	pub aspect_ratio: Option<AspectRatio>,
}

/// `pub.leaflet.richtext.facet`: inline markup over a byte range of a block's
/// `plaintext`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Facet {
	/// The byte range the features apply to.
	pub index: ByteSlice,
	/// What the range is.
	pub features: Vec<FacetFeature>,
}

/// `#byteSlice`: a range of UTF-8 bytes, start inclusive, end exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteSlice {
	/// The first byte.
	#[serde(rename = "byteStart")]
	pub byte_start: usize,
	/// One past the last byte.
	#[serde(rename = "byteEnd")]
	pub byte_end: usize,
}

/// A feature of a facet, naming its def in `$type`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum FacetFeature {
	/// A link to `uri`.
	#[serde(rename = "pub.leaflet.richtext.facet#link")]
	Link {
		/// Where the link goes.
		uri: Uri,
	},
	/// Bold text.
	#[serde(rename = "pub.leaflet.richtext.facet#bold")]
	Bold,
	/// Italic text.
	#[serde(rename = "pub.leaflet.richtext.facet#italic")]
	Italic,
	/// Inline code.
	#[serde(rename = "pub.leaflet.richtext.facet#code")]
	Code,
	/// Highlighted text.
	#[serde(rename = "pub.leaflet.richtext.facet#highlight")]
	Highlight,
	/// Underlined text.
	#[serde(rename = "pub.leaflet.richtext.facet#underline")]
	Underline,
	/// Struck text.
	#[serde(rename = "pub.leaflet.richtext.facet#strikethrough")]
	Strikethrough,
}

/// The Leaflet render target: a page's tree as [`LeafletContent`], answering
/// [`MEDIA_TYPE`](Self::MEDIA_TYPE) and embedding images, so the media
/// resolve step fetches an `img`'s bytes before it renders.
///
/// Renders the body alone, never the whole `site.standard.document`: the
/// record needs what a page cannot know (the publication's at-uri, the
/// resolved contributors, the announcement), so a publish step assembles it.
/// A render is pure over the resolved tree, so a dry run renders exactly what
/// a publish writes.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LeafletRenderer;

impl LeafletRenderer {
	/// The media type a request for Leaflet's content names.
	pub const MEDIA_TYPE: &'static str =
		"application/vnd.pub.leaflet.content+json";

	/// [`MEDIA_TYPE`](Self::MEDIA_TYPE) as a [`MediaType`].
	pub fn media_type() -> MediaType { MediaType::other(Self::MEDIA_TYPE) }
}

impl NodeRenderer for LeafletRenderer {
	fn render(
		&mut self,
		cx: &mut RenderContext,
	) -> Result<MediaBytes, RenderError> {
		cx.check_accepts(&[Self::media_type()])?;
		let root = cx.entity;
		let content = cx
			.world
			.with_state::<LeafletQuery, _>(|query| query.content(root))?;
		let bytes = MediaType::Json.serialize(&content)?;
		if bytes.len() > LeafletContent::BUDGET {
			return Err(RenderError::Other(bevyhow!(
				"the Leaflet content is {} bytes, over the {} a record holds: \
				 write its pages to a `blobPages` blob instead",
				bytes.len(),
				LeafletContent::BUDGET
			)));
		}
		MediaBytes::new(Self::media_type(), bytes).xok()
	}
}

impl RenderTarget for LeafletRenderer {
	fn media_types(&self) -> Vec<MediaType> { vec![Self::media_type()] }

	fn embeds(&self) -> Vec<MediaKind> { vec![MediaKind::Image] }
}

/// What the Leaflet walk reads off a rendered tree.
#[derive(SystemParam)]
struct LeafletQuery<'w, 's> {
	elements: Query<'w, 's, &'static Element>,
	children: Query<'w, 's, &'static Children>,
	values: Query<'w, 's, &'static Value, Without<Element>>,
	portals: Query<'w, 's, &'static Portal>,
	attributes: AttributeQuery<'w, 's>,
	inline_blobs: Query<'w, 's, &'static InlineBlob>,
	metas: Query<'w, 's, &'static PageMeta>,
	urls: Query<'w, 's, &'static PageUrl>,
	unregistered: Query<'w, 's, (), With<UnregisteredTag>>,
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

/// The tags whose content is never prose: metadata, scripts, pictures.
const SKIPPED: &[&str] = &[
	"head", "script", "style", "template", "noscript", "meta", "link", "title",
	"svg", "video", "audio", "canvas",
];

/// The inline tags whose text flows into the line with no facet of its own.
const INLINE: &[&str] = &[
	"span", "sup", "sub", "small", "abbr", "cite", "kbd", "samp", "var",
	"time", "q", "dfn", "bdi", "bdo", "wbr", "label",
];

impl LeafletQuery<'_, '_> {
	/// The document the tree at `root` renders as: the companion video, then
	/// the blocks.
	fn content(&self, root: Entity) -> Result<LeafletContent> {
		let mut blocks = Vec::new();
		if let Some(video) = self
			.metas
			.get(root)
			.ok()
			.and_then(|meta| meta.video_url.clone())
		{
			blocks.push(Self::video(&video)?);
		}
		self.blocks(root, &mut blocks)?;
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
		match PageMeta::youtube_id(url) {
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

	/// The url the page answers at, which every relative link resolves
	/// against.
	fn page_url(&self, entity: Entity) -> Url {
		self.urls
			.get(entity)
			.map(|url| url.0.clone())
			.unwrap_or_default()
	}

	/// `href` as a link a reader anywhere follows: an absolute uri verbatim,
	/// a relative one resolved against the page.
	fn absolute(&self, root: Entity, href: &str) -> Result<Uri> {
		match Uri::parse(href) {
			Ok(uri) => uri.xok(),
			Err(_) => Uri::parse(
				&self.page_url(root).join(Url::parse(href)?).to_string(),
			),
		}
	}

	/// The blocks the tree at `entity` writes, appended to `out`: block
	/// elements as their blocks, runs of inline content as text blocks.
	fn blocks(&self, entity: Entity, out: &mut Vec<LeafletBlock>) -> Result {
		let mut run = InlineRun::default();
		self.block_node(entity, entity, out, &mut run)?;
		run.flush_text(out);
		Ok(())
	}

	/// One node of a block walk from `root`: a block element flushes the
	/// pending inline `run` and writes its block, inline content extends it.
	fn block_node(
		&self,
		root: Entity,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(portal) = self.portals.get(entity) {
			return self.block_node(root, portal.target(), out, run);
		}
		if let Ok(value) = self.values.get(entity) {
			run.push_text(&value.to_string());
			return Ok(());
		}
		let Ok(element) = self.elements.get(entity) else {
			return self.block_children(root, entity, out, run);
		};
		let tag = element.tag().to_ascii_lowercase();
		match tag.as_str() {
			tag if SKIPPED.contains(&tag) => {}
			"h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
				run.flush_text(out);
				let (plaintext, facets) = self.inline_text(root, entity)?;
				out.push(LeafletBlock::Header(HeaderBlock {
					level: tag[1..].parse().unwrap_or(1),
					plaintext,
					facets,
				}));
			}
			"p" => {
				run.flush_text(out);
				self.inline_children(root, entity, out, run)?;
				run.flush_text(out);
			}
			"blockquote" => {
				run.flush_text(out);
				out.push(self.blockquote(root, entity)?);
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
					self.unordered_list(root, entity)?,
				));
			}
			"ol" => {
				run.flush_text(out);
				out.push(LeafletBlock::OrderedList(
					self.ordered_list(root, entity)?,
				));
			}
			"img" => {
				run.flush_text(out);
				out.push(self.image(root, entity)?);
			}
			"table" => {
				run.flush_text(out);
				out.push(LeafletBlock::Code(CodeBlock {
					plaintext: self.pipe_table(entity),
					language: None,
				}));
			}
			"iframe" => {
				run.flush_text(out);
				if let Some(block) = self.iframe(root, entity)? {
					out.push(block);
				}
			}
			"figure" if self.is_diagram(entity) => {
				run.flush_text(out);
				out.push(self.diagram(entity));
			}
			"br" => run.push_break(),
			tag if CONTAINERS.contains(&tag) => {
				run.flush_text(out);
				self.block_children(root, entity, out, run)?;
				run.flush_text(out);
			}
			_ if self.is_inline(entity, &tag) => {
				self.inline_node(root, entity, out, run)?;
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
		root: Entity,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(children) = self.children.get(entity) {
			for child in children.iter() {
				self.block_node(root, child, out, run)?;
			}
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
		root: Entity,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(children) = self.children.get(entity) {
			for child in children.iter() {
				self.inline_node(root, child, out, run)?;
			}
		}
		Ok(())
	}

	/// One node of an inline run: text extends it, a marked element opens a
	/// facet over what it contains, and an image splits the run around its
	/// own block.
	fn inline_node(
		&self,
		root: Entity,
		entity: Entity,
		out: &mut Vec<LeafletBlock>,
		run: &mut InlineRun,
	) -> Result {
		if let Ok(portal) = self.portals.get(entity) {
			return self.inline_node(root, portal.target(), out, run);
		}
		if let Ok(value) = self.values.get(entity) {
			run.push_text(&value.to_string());
			return Ok(());
		}
		let Ok(element) = self.elements.get(entity) else {
			return self.inline_children(root, entity, out, run);
		};
		let tag = element.tag().to_ascii_lowercase();
		match tag.as_str() {
			tag if SKIPPED.contains(&tag) => Ok(()),
			"br" => {
				run.push_break();
				Ok(())
			}
			"img" => {
				run.split(out);
				out.push(self.image(root, entity)?);
				Ok(())
			}
			"a" => {
				let feature = self
					.attribute(entity, "href")
					.map(|href| self.absolute(root, &href))
					.transpose()?
					.map(|uri| FacetFeature::Link { uri });
				run.open(feature);
				self.inline_children(root, entity, out, run)?;
				run.close();
				Ok(())
			}
			tag => {
				run.open(InlineRun::feature(tag));
				self.inline_children(root, entity, out, run)?;
				run.close();
				Ok(())
			}
		}
	}

	/// The inline text and facets of the element at `entity`, for a block
	/// that holds one run (a heading, a list item).
	fn inline_text(
		&self,
		root: Entity,
		entity: Entity,
	) -> Result<(String, Vec<Facet>)> {
		let mut run = InlineRun::default();
		let mut out = Vec::new();
		self.inline_children(root, entity, &mut out, &mut run)?;
		run.trim_end();
		(run.plaintext, run.facets).xok()
	}

	/// A quotation: its paragraphs joined by a blank line, their facets
	/// shifted along with them.
	fn blockquote(&self, root: Entity, entity: Entity) -> Result<LeafletBlock> {
		let mut inner = Vec::new();
		let mut run = InlineRun::default();
		self.block_children(root, entity, &mut inner, &mut run)?;
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
		root: Entity,
		entity: Entity,
	) -> Result<UnorderedListBlock> {
		let mut list = UnorderedListBlock::default();
		for item in self.child_elements(entity, "li") {
			let (content, nested) = self.list_item(root, item)?;
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
		root: Entity,
		entity: Entity,
	) -> Result<OrderedListBlock> {
		let mut list = OrderedListBlock {
			start_index: self
				.attribute(entity, "start")
				.and_then(|start| start.parse().ok()),
			..default()
		};
		for item in self.child_elements(entity, "li") {
			let (content, nested) = self.list_item(root, item)?;
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
		root: Entity,
		item: Entity,
	) -> Result<(TextBlock, Vec<NestedList>)> {
		let mut run = InlineRun::default();
		let mut out = Vec::new();
		let mut nested = Vec::new();
		if let Ok(children) = self.children.get(item) {
			for child in children.iter() {
				match self
					.elements
					.get(child)
					.map(|element| element.tag().to_ascii_lowercase())
					.as_deref()
				{
					Ok("ul") => nested.push(NestedList::Unordered(
						self.unordered_list(root, child)?,
					)),
					Ok("ol") => nested.push(NestedList::Ordered(
						self.ordered_list(root, child)?,
					)),
					// a loose list wraps each item's text in a paragraph
					Ok("p") => {
						run.push_text(" ");
						self.inline_children(root, child, &mut out, &mut run)?;
					}
					_ => self.inline_node(root, child, &mut out, &mut run)?,
				}
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
	fn image(&self, root: Entity, entity: Entity) -> Result<LeafletBlock> {
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
				root,
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
		root: Entity,
		entity: Entity,
	) -> Result<Option<LeafletBlock>> {
		let Some(src) = self
			.attribute(entity, "alt-src")
			.or_else(|| self.attribute(entity, "src"))
		else {
			return Ok(None);
		};
		LeafletBlock::Website(WebsiteBlock {
			src: self.absolute(root, &src)?,
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
		let mut queue = vec![table];
		while let Some(entity) = queue.pop() {
			let tag = self
				.elements
				.get(entity)
				.map(|element| element.tag().to_ascii_lowercase())
				.unwrap_or_default();
			if tag == "tr" {
				let cells = self
					.children
					.get(entity)
					.map(|children| {
						children
							.iter()
							.filter(|cell| {
								self.elements.get(*cell).is_ok_and(|element| {
									matches!(element.tag(), "td" | "th")
								})
							})
							.map(|cell| collapse_whitespace(&self.text(cell)))
							.collect()
					})
					.unwrap_or_default();
				let is_header = self.child_elements(entity, "th").len() > 0;
				if is_header && rows.len() == header_rows {
					header_rows += 1;
				}
				rows.push(cells);
				continue;
			}
			if let Ok(children) = self.children.get(entity) {
				// depth first in document order
				queue.extend(children.iter().rev());
			}
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

	/// The child elements of `entity` tagged `tag`.
	fn child_elements(&self, entity: Entity, tag: &str) -> Vec<Entity> {
		self.children
			.get(entity)
			.map(|children| {
				children
					.iter()
					.filter(|child| {
						self.elements.get(*child).is_ok_and(|element| {
							element.tag().eq_ignore_ascii_case(tag)
						})
					})
					.collect()
			})
			.unwrap_or_default()
	}

	/// Every text node under `entity` verbatim, in document order.
	fn text(&self, entity: Entity) -> String {
		let mut out = String::new();
		let mut stack = vec![entity];
		while let Some(entity) = stack.pop() {
			if let Ok(portal) = self.portals.get(entity) {
				stack.push(portal.target());
				continue;
			}
			if let Ok(value) = self.values.get(entity) {
				out.push_str(&value.to_string());
			}
			if let Ok(children) = self.children.get(entity) {
				stack.extend(children.iter().rev());
			}
		}
		out
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
	facets: Vec<Facet>,
	/// The open marks, each its feature (none for an element that only
	/// flows) and the byte it started at.
	open: Vec<(Option<FacetFeature>, usize)>,
}

impl InlineRun {
	/// The facet feature an inline `tag` marks its content with.
	fn feature(tag: &str) -> Option<FacetFeature> {
		match tag {
			"strong" | "b" => Some(FacetFeature::Bold),
			"em" | "i" => Some(FacetFeature::Italic),
			"code" => Some(FacetFeature::Code),
			"mark" => Some(FacetFeature::Highlight),
			"u" | "ins" => Some(FacetFeature::Underline),
			"s" | "del" | "strike" => Some(FacetFeature::Strikethrough),
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
	fn open(&mut self, feature: Option<FacetFeature>) {
		self.open.push((feature, self.plaintext.len()));
	}

	/// Close the innermost mark, recording its facet when it covers anything.
	fn close(&mut self) {
		if let Some((Some(feature), start)) = self.open.pop() {
			self.push_facet(feature, start, self.plaintext.len());
		}
	}

	fn push_facet(&mut self, feature: FacetFeature, start: usize, end: usize) {
		let end = end.min(self.plaintext.trim_end().len());
		if start < end {
			self.facets.push(Facet {
				index: ByteSlice {
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
	#[allow(unused_imports)]
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// A world rendering to Leaflet.
	fn world() -> World {
		let mut world = world_ext::ui_world();
		world
			.resource_mut::<RenderTargets>()
			.register(LeafletRenderer);
		world
	}

	/// The blocks the tree at `root` renders as, as json.
	fn blocks(world: &mut World, root: Entity) -> serde_json::Value {
		let bytes =
			RenderTargets::render(world, root, &LeafletRenderer::media_type())
				.unwrap();
		let content: serde_json::Value =
			serde_json::from_slice(&bytes).unwrap();
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

	/// A relative link resolves against the page's url.
	#[beet_core::test]
	fn resolves_relative_links() {
		let mut world = world();
		let root = world
			.spawn((
				PageUrl(Url::parse("https://beet.org/blog/post").unwrap()),
				rsx! { <p><a href="../docs">"docs"</a></p> },
			))
			.id();
		blocks(&mut world, root)[0]["facets"][0]["features"][0]["uri"]
			.xpect_eq(json(r#""https://beet.org/docs""#));
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
			.spawn((
				PageUrl(Url::parse("https://beet.org/blog/post").unwrap()),
				rsx! {
					<div>
						<p>"Before "<img src="./fetched.png" alt="Fetched"/>" after."</p>
						<img src="/linked.png" alt="Linked"/>
					</div>
				},
			))
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
			UnregisteredTag("Counter".into()),
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
			PageMeta {
				video_url: Some(Url::parse("https://youtu.be/7koepBSRoUI").unwrap()),
				..default()
			},
			rsx! { <p>"Body"</p> },
		))[0]
			.xpect_eq(json(
				r#"{"$type":"pub.leaflet.blocks.iframe","url":"https://www.youtube.com/embed/7koepBSRoUI","aspectRatio":{"width":16,"height":9}}"#,
			));
	}

	/// Content over a record's budget fails naming the escape hatch.
	#[beet_core::test]
	fn refuses_content_over_the_budget() {
		let mut world = world();
		let root = world
			.spawn((Element::new("p"), children![Value::from(
				"x".repeat(LeafletContent::BUDGET)
			)]))
			.id();
		RenderTargets::render(&mut world, root, &LeafletRenderer::media_type())
			.unwrap_err()
			.to_string()
			.xpect_contains("blobPages");
	}

	/// The `--root=content` Leaflet render of the site's own blog post at
	/// `slug`, read off `site/routes/blog` and rendered as a request would
	/// render it, its media linked so a run needs no assets.
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
		let body = world
			.entity_mut(router)
			.exchange(
				Request::get(format!("/{slug}"))
					.with_param("root", "content")
					.with_param("media-ingest", "link")
					.with_accept(LeafletRenderer::media_type()),
			)
			.await
			.into_result()
			.await
			.unwrap()
			.text()
			.await
			.unwrap();
		serde_json::to_string_pretty(&json(&body)).unwrap()
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

	/// The types read the wire form Leaflet's own records write and write it
	/// back unchanged.
	#[beet_core::test]
	fn reads_a_leaflet_record() {
		let record = json(LEAFLET_RECORD);
		serde_json::from_value::<LeafletContent>(record.clone())
			.unwrap()
			.xmap(|content| serde_json::to_value(content).unwrap())
			.xpect_eq(record);
	}

	/// Blocks from a `pub.leaflet.content` record leaflet.pub wrote, read
	/// 2026-10-08.
	const LEAFLET_RECORD: &str = r#"{"$type": "pub.leaflet.content", "pages": [{"id": "01a0d961-8ef2-7442-9c75-0579560d0073", "$type": "pub.leaflet.pages.linearDocument", "blocks": [{"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 56, "byteStart": 38}, "features": [{"$type": "pub.leaflet.richtext.facet#italic"}]}], "plaintext": "You may not know this, but Leaflet is all about learning."}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 54, "byteStart": 45}, "features": [{"uri": "https://hyperlink.academy/", "$type": "pub.leaflet.richtext.facet#link"}]}], "plaintext": "The three of us first worked together making Hyperlink, which evolved from indie internet school, to software for collaborative learning spaces…seeds that eventually grew into Leaflet!"}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 197, "byteStart": 162}, "features": [{"$type": "pub.leaflet.richtext.facet#highlight"}, {"$type": "pub.leaflet.richtext.facet#italic"}]}], "plaintext": "We care deeply about building tools that help people do meaningful things together. We think learning is one of the most meaningful things there is, and we think publishing tools are learning tools."}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"alt": "hand drawn monochrome pixel art of a student desk overgrown with plant life, wrapped in a tangle of vines", "$type": "pub.leaflet.blocks.image", "image": {"ref": {"$link": "bafkreiethxtad6a254thukly6tjfehwahk3l6k3grxjeox6yp2qcsdlvtu"}, "size": 38490, "$type": "blob", "mimeType": "image/webp"}, "width": 592, "aspectRatio": {"width": 1448, "height": 1074}}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.header", "level": 2, "plaintext": "Back to School Special"}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.unorderedList", "children": [{"$type": "pub.leaflet.blocks.unorderedList#listItem", "content": {"$type": "pub.leaflet.blocks.text", "plaintext": "email newsletters (up to 1k subscribers included)"}}, {"$type": "pub.leaflet.blocks.unorderedList#listItem", "content": {"$type": "pub.leaflet.blocks.text", "plaintext": "publication analytics"}}]}}]}]}"#;
}
