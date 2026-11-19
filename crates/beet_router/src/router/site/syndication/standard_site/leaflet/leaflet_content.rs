//! The `pub.leaflet.*` lexicon types beet reads and writes.
use beet_core::prelude::*;

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
/// checked on read; a block this build does not know is held whole as
/// [`Unknown`](Self::Unknown) and written back unchanged.
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
	/// A block of a def this build does not know, ie
	/// `pub.leaflet.blocks.button`.
	Unknown(OpenUnion),
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
			_ => OpenUnion::try_from(value).map(Self::Unknown),
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
	pub facets: Vec<LeafletFacet>,
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
	pub facets: Vec<LeafletFacet>,
}

/// `pub.leaflet.blocks.blockquote`: a quotation.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type", rename = "pub.leaflet.blocks.blockquote")]
pub struct BlockquoteBlock {
	/// The quotation's text, its paragraphs joined by a blank line.
	pub plaintext: String,
	/// The inline markup over [`plaintext`](Self::plaintext).
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub facets: Vec<LeafletFacet>,
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
pub struct LeafletFacet {
	/// The byte range the features apply to.
	pub index: LeafletByteSlice,
	/// What the range is.
	pub features: Vec<LeafletFacetFeature>,
}

/// `#byteSlice`: a range of UTF-8 bytes, start inclusive, end exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeafletByteSlice {
	/// The first byte.
	#[serde(rename = "byteStart")]
	pub byte_start: usize,
	/// One past the last byte.
	#[serde(rename = "byteEnd")]
	pub byte_end: usize,
}

/// A feature of a facet, naming its def in `$type`. One this build has no
/// variant for is held whole as [`Unknown`](Self::Unknown) and written back
/// unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "Value", into = "Value")]
pub enum LeafletFacetFeature {
	/// A link to `uri`.
	Link {
		/// Where the link goes.
		uri: Uri,
	},
	/// Bold text.
	Bold,
	/// Italic text.
	Italic,
	/// Inline code.
	Code,
	/// Highlighted text.
	Highlight,
	/// Underlined text.
	Underline,
	/// Struck text.
	Strikethrough,
	/// A feature of a def this build does not know, ie
	/// `pub.leaflet.richtext.facet#didMention`.
	Unknown(OpenUnion),
}

impl LeafletFacetFeature {
	/// The def this feature names in `$type`.
	pub fn nsid(&self) -> Nsid {
		match self {
			Self::Link { .. } => {
				Nsid::new_static("pub.leaflet.richtext.facet#link")
			}
			Self::Bold => Nsid::new_static("pub.leaflet.richtext.facet#bold"),
			Self::Italic => {
				Nsid::new_static("pub.leaflet.richtext.facet#italic")
			}
			Self::Code => Nsid::new_static("pub.leaflet.richtext.facet#code"),
			Self::Highlight => {
				Nsid::new_static("pub.leaflet.richtext.facet#highlight")
			}
			Self::Underline => {
				Nsid::new_static("pub.leaflet.richtext.facet#underline")
			}
			Self::Strikethrough => {
				Nsid::new_static("pub.leaflet.richtext.facet#strikethrough")
			}
			Self::Unknown(member) => member.r#type().clone(),
		}
	}
}

/// The feature as written, `$type` first.
impl From<LeafletFacetFeature> for Value {
	fn from(feature: LeafletFacetFeature) -> Self {
		let name = feature.nsid().to_string();
		match feature {
			LeafletFacetFeature::Unknown(member) => member.into(),
			LeafletFacetFeature::Link { uri } => {
				let uri = uri.to_string();
				value!({ "$type": name, "uri": uri })
			}
			_ => value!({ "$type": name }),
		}
	}
}

/// A feature read by its `$type`.
impl TryFrom<Value> for LeafletFacetFeature {
	type Error = BevyError;
	fn try_from(value: Value) -> Result<Self> {
		let member = OpenUnion::try_from(value)?;
		match member.r#type().as_str() {
			"pub.leaflet.richtext.facet#link" => Self::Link {
				uri: Uri::parse(member.fields().get("uri")?.as_str()?)?,
			},
			"pub.leaflet.richtext.facet#bold" => Self::Bold,
			"pub.leaflet.richtext.facet#italic" => Self::Italic,
			"pub.leaflet.richtext.facet#code" => Self::Code,
			"pub.leaflet.richtext.facet#highlight" => Self::Highlight,
			"pub.leaflet.richtext.facet#underline" => Self::Underline,
			"pub.leaflet.richtext.facet#strikethrough" => Self::Strikethrough,
			_ => Self::Unknown(member),
		}
		.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The types read the wire form Leaflet's own records write and write it
	/// back unchanged.
	#[beet_core::test]
	fn reads_a_leaflet_record() {
		let record: serde_json::Value =
			serde_json::from_str(LEAFLET_RECORD).unwrap();
		serde_json::from_value::<LeafletContent>(record.clone())
			.unwrap()
			.xmap(|content| serde_json::to_value(content).unwrap())
			.xpect_eq(record);
	}

	/// Blocks from a `pub.leaflet.content` record leaflet.pub wrote, read
	/// 2026-10-08.
	const LEAFLET_RECORD: &str = r#"{"$type": "pub.leaflet.content", "pages": [{"id": "01a0d961-8ef2-7442-9c75-0579560d0073", "$type": "pub.leaflet.pages.linearDocument", "blocks": [{"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 56, "byteStart": 38}, "features": [{"$type": "pub.leaflet.richtext.facet#italic"}]}], "plaintext": "You may not know this, but Leaflet is all about learning."}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 54, "byteStart": 45}, "features": [{"uri": "https://hyperlink.academy/", "$type": "pub.leaflet.richtext.facet#link"}]}], "plaintext": "The three of us first worked together making Hyperlink, which evolved from indie internet school, to software for collaborative learning spaces…seeds that eventually grew into Leaflet!"}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.text", "facets": [{"index": {"byteEnd": 197, "byteStart": 162}, "features": [{"$type": "pub.leaflet.richtext.facet#highlight"}, {"$type": "pub.leaflet.richtext.facet#italic"}]}], "plaintext": "We care deeply about building tools that help people do meaningful things together. We think learning is one of the most meaningful things there is, and we think publishing tools are learning tools."}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"alt": "hand drawn monochrome pixel art of a student desk overgrown with plant life, wrapped in a tangle of vines", "$type": "pub.leaflet.blocks.image", "image": {"ref": {"$link": "bafkreiethxtad6a254thukly6tjfehwahk3l6k3grxjeox6yp2qcsdlvtu"}, "size": 38490, "$type": "blob", "mimeType": "image/webp"}, "width": 592, "aspectRatio": {"width": 1448, "height": 1074}}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.header", "level": 2, "plaintext": "Back to School Special"}}, {"$type": "pub.leaflet.pages.linearDocument#block", "block": {"$type": "pub.leaflet.blocks.unorderedList", "children": [{"$type": "pub.leaflet.blocks.unorderedList#listItem", "content": {"$type": "pub.leaflet.blocks.text", "plaintext": "email newsletters (up to 1k subscribers included)"}}, {"$type": "pub.leaflet.blocks.unorderedList#listItem", "content": {"$type": "pub.leaflet.blocks.text", "plaintext": "publication analytics"}}]}}]}]}"#;
}
