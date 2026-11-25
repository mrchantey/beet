//! The Leaflet parsers: a `pub.leaflet.content` member read onto entities.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_ui::prelude::*;

/// A [`LexiconParser`] of one Leaflet def, reading it into the elements the
/// markdown parser produces (`Element`, `Value`, `Attribute`, `ChildOf`), so
/// the html renderer renders a Leaflet post knowing nothing of Leaflet.
///
/// Registered per def, so a reader degrades block by block: a block no
/// parser reads lands whole as an [`UnknownLexicon`], which
/// [`LeafletRenderer`] writes back verbatim in its slot. Install every def
/// with [`LeafletParser::all`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeafletParser {
	def: LeafletDef,
}

impl LeafletParser {
	/// A parser of every def beet reads, installed together.
	pub fn all() -> impl Installer {
		|app: &mut App| {
			for def in LeafletDef::ALL {
				app.install(Self { def });
			}
		}
	}
}

impl LexiconParser for LeafletParser {
	fn nsid(&self) -> Nsid { self.def.nsid() }

	fn parse(&mut self, cx: LexiconParseContext) -> Result {
		match self.def {
			LeafletDef::Content => Self::content(cx),
			LeafletDef::LinearDocument => Self::linear_document(cx),
			LeafletDef::Text => Self::text(cx),
			LeafletDef::Header => Self::header(cx),
			LeafletDef::Blockquote => Self::blockquote(cx),
			LeafletDef::Code => Self::code(cx),
			LeafletDef::HorizontalRule => {
				cx.entity.insert(Element::new("hr"));
				Ok(())
			}
			LeafletDef::UnorderedList => Self::list(cx, "ul"),
			LeafletDef::OrderedList => Self::list(cx, "ol"),
			LeafletDef::UnorderedListItem => {
				Self::list_item(cx, "orderedListChildren")
			}
			LeafletDef::OrderedListItem => {
				Self::list_item(cx, "unorderedListChildren")
			}
			LeafletDef::Image => Self::image(cx),
			LeafletDef::Website => Self::website(cx),
			LeafletDef::Iframe => Self::iframe(cx),
		}
	}
}

impl Installer for LeafletParser {
	fn install(self, app: &mut App) { LexiconParsers::register_on(app, self); }
}

impl LeafletParser {
	/// The document: each page a child, in order. An image with no
	/// [`LexiconSource`] to address its blob is warned once here, not per
	/// image.
	fn content(mut cx: LexiconParseContext) -> Result {
		let member = cx.member;
		for page in member.fields().get("pages")?.as_list()? {
			cx.parse_child(&OpenUnion::try_from(page.clone())?)?;
		}
		if cx.source.is_none() {
			let images = member
				.fields()
				.0
				.values()
				.map(|value| count_members(value, &IMAGE))
				.sum::<usize>();
			if images > 0 {
				warn!(
					"leaflet: {images} images have no `src`, the document was \
					 read with no `LexiconSource` to address their blobs"
				);
			}
		}
		Ok(())
	}

	/// A page: each slot's block a child, in order. A slot is a wrapper the
	/// page reads itself, never dispatched.
	fn linear_document(mut cx: LexiconParseContext) -> Result {
		let member = cx.member;
		for slot in member.fields().get("blocks")?.as_list()? {
			let block = slot.as_map()?.get("block")?;
			cx.parse_child(&OpenUnion::try_from(block.clone())?)?;
		}
		Ok(())
	}

	/// A paragraph.
	fn text(cx: LexiconParseContext) -> Result {
		let text: TextBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("p"));
		RichText::new(text.plaintext, text.facets).spawn_into(cx.entity)
	}

	/// A heading, its level clamped to the six html has.
	fn header(cx: LexiconParseContext) -> Result {
		let header: HeaderBlock = cx.member.clone().into_serde()?;
		let level = header.level.clamp(1, 6);
		cx.entity.insert(Element::new(format!("h{level}")));
		RichText::new(header.plaintext, header.facets).spawn_into(cx.entity)
	}

	/// A quotation, a paragraph per blank-line-separated run of its text, as
	/// the renderer joined them.
	fn blockquote(cx: LexiconParseContext) -> Result {
		let quote: BlockquoteBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("blockquote"));
		let parent = cx.entity.id();
		let paragraphs =
			RichText::new(quote.plaintext, quote.facets).paragraphs();
		cx.entity.world_scope(|world| {
			for paragraph in paragraphs {
				let mut entity =
					world.spawn((ChildOf(parent), Element::new("p")));
				paragraph.spawn_into(&mut entity)?;
			}
			Ok(())
		})
	}

	/// A code block as `pre > code`, its language the code's bare class and
	/// its text ending in the newline a fence's does.
	fn code(cx: LexiconParseContext) -> Result {
		let code: CodeBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("pre"));
		let mut text = code.plaintext;
		if !text.is_empty() && !text.ends_with('\n') {
			text.push('\n');
		}
		let pre = cx.entity.id();
		cx.entity.world_scope(|world| {
			let element =
				world.spawn((ChildOf(pre), Element::new("code"))).id();
			if let Some(language) = code.language {
				spawn_attribute(world, element, "class", language);
			}
			world.spawn((ChildOf(element), Value::from(text)));
		});
		Ok(())
	}

	/// A list, each item a child dispatched by its own def.
	fn list(mut cx: LexiconParseContext, tag: &str) -> Result {
		cx.entity.insert(Element::new(tag));
		let member = cx.member;
		if let Ok(start) = member.fields().get("startIndex") {
			let start = start.as_u64()?.to_string();
			let entity = cx.entity.id();
			cx.entity.world_scope(|world| {
				spawn_attribute(world, entity, "start", start)
			});
		}
		for item in member.fields().get("children")?.as_list()? {
			cx.parse_child(&OpenUnion::try_from(item.clone())?)?;
		}
		Ok(())
	}

	/// A list item: its text inline, as a tight markdown list writes it, then
	/// a nested list of its own kind (`children`) and one of the other kind
	/// (`other`).
	fn list_item(mut cx: LexiconParseContext, other: &str) -> Result {
		cx.entity.insert(Element::new("li"));
		let member = cx.member;
		let content =
			OpenUnion::try_from(member.fields().get("content")?.clone())?;
		if content.r#type() == &TEXT {
			let text: TextBlock = content.into_serde()?;
			RichText::new(text.plaintext, text.facets).spawn_into(cx.entity)?;
		} else {
			cx.parse_child(&content)?;
		}
		// the same kind nests as bare items, read as the list they make
		let children = member
			.fields()
			.get("children")
			.ok()
			.and_then(|children| children.as_list().ok())
			.cloned()
			.unwrap_or_default();
		if !children.is_empty() {
			let list = if member.r#type() == &UNORDERED_LIST_ITEM {
				UNORDERED_LIST
			} else {
				ORDERED_LIST
			};
			cx.parse_child(&OpenUnion::new(
				list,
				value!({ "children": children }),
			)?)?;
		}
		if let Ok(nested) = member.fields().get(other) {
			cx.parse_child(&OpenUnion::try_from(nested.clone())?)?;
		}
		Ok(())
	}

	/// An image: an `img` whose `src` is its blob on the repo the record was
	/// read from, and no `src` when that repo is unknown.
	fn image(cx: LexiconParseContext) -> Result {
		let image: ImageBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("img"));
		let entity = cx.entity.id();
		let src = cx.source.map(|source| source.blob_url(&image.image.cid));
		if src.is_none() {
			debug!("leaflet: an image with no source, written with no `src`");
		}
		cx.entity.world_scope(|world| {
			if let Some(src) = src {
				spawn_attribute(world, entity, "src", src.to_string());
			}
			if let Some(alt) = image.alt {
				spawn_attribute(world, entity, "alt", alt);
			}
		});
		Ok(())
	}

	/// A link card as a paragraph holding its link, titled when it has one.
	fn website(cx: LexiconParseContext) -> Result {
		let website: WebsiteBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("p"));
		let paragraph = cx.entity.id();
		let href = website.src.to_string();
		let text = website.title.unwrap_or_else(|| href.clone());
		cx.entity.world_scope(|world| {
			let link =
				world.spawn((ChildOf(paragraph), Element::new("a"))).id();
			spawn_attribute(world, link, "href", href);
			world.spawn((ChildOf(link), Value::from(text)));
		});
		Ok(())
	}

	/// An embedded page.
	fn iframe(cx: LexiconParseContext) -> Result {
		let iframe: IframeBlock = cx.member.clone().into_serde()?;
		cx.entity.insert(Element::new("iframe"));
		let entity = cx.entity.id();
		cx.entity.world_scope(|world| {
			spawn_attribute(world, entity, "src", iframe.url.to_string())
		});
		Ok(())
	}
}

/// Every Leaflet def beet reads, one [`LeafletParser`] each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeafletDef {
	Content,
	LinearDocument,
	Text,
	Header,
	Blockquote,
	Code,
	HorizontalRule,
	UnorderedList,
	OrderedList,
	UnorderedListItem,
	OrderedListItem,
	Image,
	Website,
	Iframe,
}

const TEXT: Nsid = Nsid::new_static("pub.leaflet.blocks.text");
const IMAGE: Nsid = Nsid::new_static("pub.leaflet.blocks.image");
const UNORDERED_LIST: Nsid =
	Nsid::new_static("pub.leaflet.blocks.unorderedList");
const ORDERED_LIST: Nsid = Nsid::new_static("pub.leaflet.blocks.orderedList");
const UNORDERED_LIST_ITEM: Nsid =
	Nsid::new_static("pub.leaflet.blocks.unorderedList#listItem");

impl LeafletDef {
	const ALL: [Self; 14] = [
		Self::Content,
		Self::LinearDocument,
		Self::Text,
		Self::Header,
		Self::Blockquote,
		Self::Code,
		Self::HorizontalRule,
		Self::UnorderedList,
		Self::OrderedList,
		Self::UnorderedListItem,
		Self::OrderedListItem,
		Self::Image,
		Self::Website,
		Self::Iframe,
	];

	fn nsid(self) -> Nsid {
		match self {
			Self::Content => LeafletContent::NSID,
			Self::LinearDocument => {
				Nsid::new_static("pub.leaflet.pages.linearDocument")
			}
			Self::Text => TEXT,
			Self::Header => Nsid::new_static("pub.leaflet.blocks.header"),
			Self::Blockquote => {
				Nsid::new_static("pub.leaflet.blocks.blockquote")
			}
			Self::Code => Nsid::new_static("pub.leaflet.blocks.code"),
			Self::HorizontalRule => {
				Nsid::new_static("pub.leaflet.blocks.horizontalRule")
			}
			Self::UnorderedList => UNORDERED_LIST,
			Self::OrderedList => ORDERED_LIST,
			Self::UnorderedListItem => UNORDERED_LIST_ITEM,
			Self::OrderedListItem => {
				Nsid::new_static("pub.leaflet.blocks.orderedList#listItem")
			}
			Self::Image => IMAGE,
			Self::Website => Nsid::new_static("pub.leaflet.blocks.website"),
			Self::Iframe => Nsid::new_static("pub.leaflet.blocks.iframe"),
		}
	}
}

/// A block's `plaintext` and the marks its facets lay over it, one mark per
/// facet feature.
struct RichText {
	plaintext: String,
	marks: Vec<Mark>,
}

/// One facet feature over a byte range of a [`RichText`], as the inline
/// element the markdown parser writes for the same markup.
#[derive(Clone)]
struct Mark {
	start: usize,
	end: usize,
	tag: &'static str,
	/// A link's target.
	href: Option<String>,
}

impl Mark {
	/// Whether `other` is the same element over a range touching this one.
	fn joins(&self, other: &Mark) -> bool {
		self.tag == other.tag
			&& self.href == other.href
			&& self.start <= other.end
			&& other.start <= self.end
	}
}

impl RichText {
	/// The marks of `facets` over `plaintext`, a range clamped into the text
	/// at char boundaries, and two of one element touching or overlapping
	/// joined, as Leaflet splits one run of italic around a link inside it. A
	/// feature beet has no element for (`#didMention`, `#atMention`,
	/// `#footnote`, `#id`) keeps its text unmarked.
	fn new(plaintext: String, facets: Vec<LeafletFacet>) -> Self {
		let boundary = |byte: usize| {
			let mut byte = byte.min(plaintext.len());
			while !plaintext.is_char_boundary(byte) {
				byte -= 1;
			}
			byte
		};
		let mut marks = Vec::new();
		for facet in facets {
			let (start, end) = (
				boundary(facet.index.byte_start),
				boundary(facet.index.byte_end),
			);
			for feature in facet.features.into_iter().filter(|_| start < end) {
				let (tag, href) = match feature {
					LeafletFacetFeature::Link { uri } => {
						("a", Some(uri.to_string()))
					}
					LeafletFacetFeature::Bold => ("strong", None),
					LeafletFacetFeature::Italic => ("em", None),
					LeafletFacetFeature::Code => ("code", None),
					LeafletFacetFeature::Highlight => ("mark", None),
					LeafletFacetFeature::Underline => ("u", None),
					LeafletFacetFeature::Strikethrough => ("del", None),
					LeafletFacetFeature::Unknown(member) => {
						debug!(
							"leaflet: a `{}` facet has no element, its text kept",
							member.r#type()
						);
						continue;
					}
				};
				let mark = Mark {
					start,
					end,
					tag,
					href,
				};
				match marks
					.iter_mut()
					.find(|other: &&mut Mark| other.joins(&mark))
				{
					Some(other) => {
						other.start = other.start.min(mark.start);
						other.end = other.end.max(mark.end);
					}
					None => marks.push(mark),
				}
			}
		}
		Self { plaintext, marks }
	}

	/// The text split at each blank line, the marks of each paragraph
	/// clamped into it and shifted to its start.
	fn paragraphs(self) -> Vec<Self> {
		let mut start = 0;
		let mut paragraphs = Vec::new();
		for text in self.plaintext.split("\n\n") {
			let end = start + text.len();
			let marks = self
				.marks
				.iter()
				.filter(|mark| mark.start < end && mark.end > start)
				.map(|mark| Mark {
					start: mark.start.max(start) - start,
					end: mark.end.min(end) - start,
					..mark.clone()
				})
				.collect();
			paragraphs.push(Self {
				plaintext: text.to_string(),
				marks,
			});
			start = end + 2;
		}
		paragraphs
	}

	/// Spawn the text under `entity` as inline elements: each stretch between
	/// mark boundaries is text under the marks covering it, a mark opening
	/// outside any that ends before it, so nested facets nest and crossing
	/// ones split. A newline is a `br`.
	fn spawn_into(&self, entity: &mut EntityWorldMut) -> Result {
		let root = entity.id();
		let mut bounds = self
			.marks
			.iter()
			.flat_map(|mark| [mark.start, mark.end])
			.chain([0, self.plaintext.len()])
			.collect::<Vec<_>>();
		bounds.sort_unstable();
		bounds.dedup();
		entity.world_scope(|world| {
			// the open marks, outermost first, each its index and element
			let mut open: Vec<(usize, Entity)> = Vec::new();
			for window in bounds.windows(2) {
				let (start, end) = (window[0], window[1]);
				let covers = |index: usize| {
					self.marks[index].start <= start
						&& self.marks[index].end >= end
				};
				if let Some(first_closed) =
					open.iter().position(|(index, _)| !covers(*index))
				{
					open.truncate(first_closed);
				}
				let mut opening = (0..self.marks.len())
					.filter(|index| {
						covers(*index)
							&& !open.iter().any(|(open, _)| open == index)
					})
					.collect::<Vec<_>>();
				opening.sort_by_key(|index| {
					(core::cmp::Reverse(self.marks[*index].end), *index)
				});
				for index in opening {
					let parent =
						open.last().map_or(root, |(_, entity)| *entity);
					let element =
						Self::spawn_mark(world, parent, &self.marks[index]);
					open.push((index, element));
				}
				let parent = open.last().map_or(root, |(_, entity)| *entity);
				Self::spawn_text(world, parent, &self.plaintext[start..end]);
			}
		});
		Ok(())
	}

	/// The element of `mark` under `parent`.
	fn spawn_mark(world: &mut World, parent: Entity, mark: &Mark) -> Entity {
		let element =
			world.spawn((ChildOf(parent), Element::new(mark.tag))).id();
		if let Some(href) = &mark.href {
			spawn_attribute(world, element, "href", href.clone());
		}
		element
	}

	/// `text` as text nodes under `parent`, each newline a `br`.
	fn spawn_text(world: &mut World, parent: Entity, text: &str) {
		for (index, line) in text.split('\n').enumerate() {
			if index > 0 {
				world.spawn((ChildOf(parent), Element::new("br")));
			}
			if !line.is_empty() {
				world.spawn((ChildOf(parent), Value::from(line)));
			}
		}
	}
}

/// The attribute `key` of `value` on the element at `entity`.
fn spawn_attribute(
	world: &mut World,
	entity: Entity,
	key: &str,
	value: impl Into<Value>,
) {
	world.spawn((AttributeOf::new(entity), Attribute::new(key), value.into()));
}

/// How many members of def `nsid` `value` holds, at any depth.
fn count_members(value: &Value, nsid: &Nsid) -> usize {
	match value {
		Value::Map(map) => {
			let own = map
				.get(OpenUnion::TYPE_KEY)
				.ok()
				.and_then(|r#type| r#type.as_str().ok())
				.is_some_and(|r#type| r#type == nsid.as_str());
			usize::from(own)
				+ map
					.0
					.values()
					.map(|value| count_members(value, nsid))
					.sum::<usize>()
		}
		Value::List(list) => {
			list.iter().map(|value| count_members(value, nsid)).sum()
		}
		_ => 0,
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_scene::prelude::*;
	use beet_ui::prelude::*;

	/// `pub.leaflet.content` as Leaflet's own publication wrote it: "Newsletter
	/// Mode", `at://did:plc:btxrwcaeyodrap5mnjw2fvmz/site.standard.document/3mleufqg5gc2x`,
	/// read 2026-11-19. It carries what beet never writes: a
	/// `blocks.button`, empty spacer paragraphs, `#didMention` and
	/// `#atMention` facets, and a facet of two features.
	const NEWSLETTER_MODE: &str =
		include_str!("../fixtures/leaflet_newsletter_mode.json");

	/// The repo [`NEWSLETTER_MODE`] was read from.
	fn leaflet_lab() -> LexiconSource {
		LexiconSource {
			did: Did::parse("did:plc:btxrwcaeyodrap5mnjw2fvmz").unwrap(),
			pds: Url::parse("https://chanterelle.us-west.host.bsky.network")
				.unwrap(),
		}
	}

	/// A world reading and writing Leaflet, its site at `https://beet.org`.
	fn world() -> World {
		let mut app = world_ext::ui_app();
		app.install((LeafletRenderer, LeafletParser::all()))
			.insert_resource(PackageConfig {
				homepage: Some(Url::parse("https://beet.org").unwrap()),
				..default()
			});
		core::mem::take(app.world_mut())
	}

	/// `json` parsed onto a fresh entity of `world`.
	fn parse(
		world: &mut World,
		json: &str,
		source: Option<&LexiconSource>,
	) -> Entity {
		let member = MediaType::Json
			.deserialize::<Value>(json.as_bytes())
			.unwrap();
		parse_value(world, member, source)
	}

	/// `member` parsed onto a fresh entity of `world`.
	fn parse_value(
		world: &mut World,
		member: Value,
		source: Option<&LexiconSource>,
	) -> Entity {
		let member = OpenUnion::try_from(member).unwrap();
		let mut entity = world.spawn_empty();
		LexiconParsers::parse(&mut entity, &member, source).unwrap();
		entity.id()
	}

	/// The tree at `entity` as html, pretty when a person reads it.
	fn html(world: &mut World, entity: Entity, pretty: bool) -> String {
		let renderer = HtmlRenderer::default();
		match pretty {
			true => renderer.pretty(),
			false => renderer,
		}
		.render(&mut RenderContext::new(
			world,
			entity,
			&RequestParts::default(),
		))
		.unwrap()
		.to_string()
	}

	/// The tree at `entity` as Leaflet's content, answering a request for
	/// `/blog/post`.
	fn render(world: &mut World, entity: Entity) -> OpenUnion {
		LexiconRenderers::render(
			world,
			entity,
			&RequestParts::get("/blog/post"),
			&LeafletContent::NSID,
		)
		.unwrap()
	}

	/// The blocks of `json`, a `pub.leaflet.content`, that this build does
	/// not know.
	fn unknown_blocks(json: &[u8]) -> Vec<OpenUnion> {
		MediaType::Json
			.deserialize::<LeafletContent>(json)
			.unwrap()
			.pages
			.into_iter()
			.flat_map(|page| page.blocks)
			.filter_map(|slot| match slot.block {
				LeafletBlock::Unknown(member) => Some(member),
				_ => None,
			})
			.collect()
	}

	/// A post exercising every block the renderer writes.
	#[cfg(feature = "markdown_parser")]
	const POST: &str = r#"# Full Stack Bevy

A paragraph with *emphasis*, **strong *and nested***, `code` and [a link](../docs).

## Lists

- one
  - one a
- two

3. three
4. four

> A quote with *emphasis*.
>
> A second paragraph.

```rust
fn main() { println!("hello"); }
```

---

![a diagram](/assets/diagram.png)

| Name | Kind |
| --- | --- |
| beet | engine |
"#;

	/// A markdown post rendered to Leaflet then read back renders the html
	/// snapshotted beside the original's. It loses only what Leaflet's block
	/// model cannot say:
	///
	/// - the `div` the markdown parser wraps an item's text in beside a nested
	///   list, since a `#listItem`'s text is a block of its own
	/// - an image the render did not fetch, written as a link card, so read
	///   back as a `p > a` to its url titled by its alt
	/// - a table, written as a pipe table in a code block
	/// - a relative link, written absolute against the site's homepage
	#[cfg(feature = "markdown_parser")]
	#[beet_core::test]
	fn round_trips_a_markdown_post() {
		let mut source = world();
		let post = source.spawn_empty().id();
		ParseContext::new(
			&mut source.entity_mut(post),
			&MediaBytes::new_markdown(POST),
		)
		.parse()
		.unwrap();
		let original = html(&mut source, post, true);
		let member = Value::from(render(&mut source, post));

		let mut read = world();
		let entity = parse_value(&mut read, member, None);
		let round_trip = html(&mut read, entity, false);
		// what Leaflet does say survives untouched
		for kept in [
			"<h1>Full Stack Bevy</h1>",
			"<em>emphasis</em>",
			"<strong>strong <em>and nested</em></strong>",
			"<code>code</code>",
			"<a href=\"https://beet.org/docs\">a link</a>",
			"<code class=\"rust\">",
			"<hr />",
		] {
			round_trip.xref().xpect_contains(kept);
		}
		let round_trip = html(&mut read, entity, true);
		format!("{original}\n---- round trip ----\n\n{round_trip}")
			.xpect_snapshot();
	}

	/// A document Leaflet wrote renders as html, its images on the repo it
	/// was read from.
	#[beet_core::test]
	fn reads_a_leaflet_document() {
		let mut world = world();
		let entity = parse(&mut world, NEWSLETTER_MODE, Some(&leaflet_lab()));
		html(&mut world, entity, false).xpect_contains(
			"<img src=\"https://chanterelle.us-west.host.bsky.network/xrpc/com.atproto.sync.getBlob?",
		);
		html(&mut world, entity, true).xpect_snapshot();
	}

	/// Read with no source, an image keeps its alt and has no `src`.
	#[beet_core::test]
	fn an_image_without_a_source_has_no_src() {
		let mut world = world();
		let entity = parse(&mut world, NEWSLETTER_MODE, None);
		html(&mut world, entity, false)
			.xref()
			.xpect_contains("<img />")
			.xnot()
			.xpect_contains("getBlob");
	}

	/// A block beet does not know, Leaflet's `blocks.button`, is read
	/// whole and rendered back unchanged.
	#[beet_core::test]
	fn an_unknown_block_survives_a_round_trip() {
		let mut world = world();
		let entity = parse(&mut world, NEWSLETTER_MODE, Some(&leaflet_lab()));
		let written = Value::from(render(&mut world, entity))
			.xmap(|value| MediaType::Json.serialize(&value))
			.unwrap();
		let unknown = unknown_blocks(NEWSLETTER_MODE.as_bytes());
		unknown
			.iter()
			.map(|member| member.r#type().to_string())
			.collect::<Vec<_>>()
			.xpect_eq(vec!["pub.leaflet.blocks.button".to_string()]);
		unknown_blocks(&written).xpect_eq(unknown);
	}

	/// Nested facets nest, crossing ones split, offsets are UTF-8 bytes, and
	/// a feature with no element keeps its text.
	#[beet_core::test]
	fn facets_become_inline_elements() {
		let mut world = world();
		let entity = parse(
			&mut world,
			r#"{"$type":"pub.leaflet.blocks.text","plaintext":"Grüße bold both\nnext @who","facets":[
				{"index":{"byteStart":8,"byteEnd":17},"features":[{"$type":"pub.leaflet.richtext.facet#bold"}]},
				{"index":{"byteStart":13,"byteEnd":22},"features":[{"$type":"pub.leaflet.richtext.facet#italic"}]},
				{"index":{"byteStart":23,"byteEnd":27},"features":[{"$type":"pub.leaflet.richtext.facet#didMention","did":"did:plc:x"}]}
			]}"#,
			None,
		);
		html(&mut world, entity, false).xpect_eq(
			"<p>Grüße <strong>bold <em>both</em></strong><em><br />next</em> @who</p>",
		);
	}
}
