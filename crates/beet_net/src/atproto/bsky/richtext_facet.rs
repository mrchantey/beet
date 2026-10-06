//! `app.bsky.richtext.facet`, the live ranges of a post's text.
use beet_core::prelude::*;

/// `app.bsky.richtext.facet`: one live range of a post's text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facet {
	/// The range, in bytes of the UTF-8 text.
	pub index: ByteSlice,
	/// What the range is.
	pub features: Vec<FacetFeature>,
}

/// `app.bsky.richtext.facet#byteSlice`: a byte range of a post's UTF-8 text,
/// end exclusive: bytes, never chars or graphemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteSlice {
	/// The first byte.
	#[serde(rename = "byteStart")]
	pub byte_start: usize,
	/// One past the last byte.
	#[serde(rename = "byteEnd")]
	pub byte_end: usize,
}

/// What a facet's range is, the union of `app.bsky.richtext.facet`'s
/// `#link`, `#mention` and `#tag`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum FacetFeature {
	/// A link to `uri`.
	#[serde(rename = "app.bsky.richtext.facet#link")]
	Link {
		/// The link target.
		uri: SmolStr,
	},
	/// A mention of the account `did`.
	#[serde(rename = "app.bsky.richtext.facet#mention")]
	Mention {
		/// The mentioned account.
		did: Did,
	},
	/// A hashtag, `tag` without its `#`.
	#[serde(rename = "app.bsky.richtext.facet#tag")]
	Tag {
		/// The tag text.
		tag: SmolStr,
	},
	/// A feature this build does not know, kept so a foreign post still reads.
	#[serde(other)]
	Other,
}
