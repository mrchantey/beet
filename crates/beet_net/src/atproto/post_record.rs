//! `app.bsky.feed.post`, the Bluesky record shape beet reads and writes.
use crate::prelude::*;
use beet_core::prelude::*;

/// An `app.bsky.feed.post` record: the text, the facets that make parts of it
/// live, and what it replies to or embeds.
///
/// A foreign record keyed by TID, so it carries the rkey it was minted with
/// and never serializes it: the address does.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostRecord {
	/// The key this post lives at, minted on first write.
	#[serde(skip)]
	pub rkey: Rkey,
	/// The post text, plain: a link or a mention is only live when a facet
	/// names its byte range. Defaulted on read, like `created_at`, so a
	/// hydrated view trimmed of its record still reads.
	#[serde(default)]
	pub text: String,
	/// When the author wrote it, an RFC 3339 datetime.
	#[serde(default, rename = "createdAt")]
	pub created_at: SmolStr,
	/// The live ranges of `text`.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub facets: Vec<Facet>,
	/// The languages the text is in, ie `en`.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub langs: Vec<SmolStr>,
	/// The thread this post replies into.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub reply: Option<ReplyRef>,
	/// The open union of embeds, ie an `app.bsky.embed.external` link card,
	/// carried as its json with its `$type`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub embed: Option<Value>,
}

impl AtprotoRecord for PostRecord {
	const COLLECTION: Nsid = Nsid::new_static("app.bsky.feed.post");
	fn rkey(&self) -> Rkey { self.rkey.clone() }
}

impl PostRecord {
	/// A post of `rich` text at `rkey`, written at `created_at`.
	pub fn new(rkey: Rkey, rich: RichText, created_at: Timestamp) -> Self {
		Self {
			rkey,
			text: rich.text,
			facets: rich.facets,
			created_at: created_at.format_iso8601().into(),
			..default()
		}
	}
}

/// Where a reply sits: the thread's first post and the one it answers.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyRef {
	/// The thread's first post.
	pub root: StrongRef,
	/// The post this one answers.
	pub parent: StrongRef,
}

/// One live range of a post's text, `app.bsky.richtext.facet`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facet {
	/// The range, in bytes of the UTF-8 text.
	pub index: ByteSlice,
	/// What the range is.
	pub features: Vec<FacetFeature>,
}

/// A byte range of a post's UTF-8 text, end exclusive: bytes, never chars or
/// graphemes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteSlice {
	/// The first byte.
	#[serde(rename = "byteStart")]
	pub byte_start: usize,
	/// One past the last byte.
	#[serde(rename = "byteEnd")]
	pub byte_end: usize,
}

/// What a facet's range is.
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

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The live reply post the record cid fixture is, reading back to the
	/// same body.
	#[beet_core::test]
	fn round_trips_a_pds_post() {
		let json = r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"hi"}"#;
		let value = Value::from_json(serde_json::from_str(json).unwrap());
		let post = value.clone().into_serde::<PostRecord>().unwrap();
		post.reply
			.as_ref()
			.unwrap()
			.root
			.uri
			.rkey()
			.as_str()
			.xpect_eq("3mw5t5lac4k2y");
		RecordEntry::typed_body(
			&PostRecord::COLLECTION,
			Value::from_serde(&post).unwrap(),
		)
		.unwrap()
		.xmap(|body| dag_cbor_ext::record_cid(&body).unwrap())
		.xpect_eq(dag_cbor_ext::record_cid(&value).unwrap());
	}

	/// A post with facets and a reply survives the repo's `Value` path, so
	/// writing it twice is one create and one no-op.
	#[beet_core::test]
	async fn a_post_converges() {
		let pds = Pds::temp();
		let post = async || {
			let rich =
				RichText::new("🤘 #bevy at https://beet.org", async |_| {
					bevybail!("no mentions")
				})
				.await
				.unwrap();
			let parent = StrongRef::new(
				AtUri::parse(
					"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y",
				)
				.unwrap(),
				Cid::parse(
					"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
				)
				.unwrap(),
			);
			PostRecord {
				langs: vec!["en".into()],
				reply: Some(ReplyRef {
					root: parent.clone(),
					parent,
				}),
				..PostRecord::new(
					Rkey::from(Tid::from(Timestamp::from_millis(1))),
					rich,
					Timestamp::from_millis(1),
				)
			}
		};
		pds.converge([post().await], &default())
			.await
			.unwrap()
			.created
			.len()
			.xpect_eq(1);
		pds.converge([post().await], &default())
			.await
			.unwrap()
			.is_noop()
			.xpect_true();
	}
}
