//! `app.bsky.feed.post`, a Bluesky post.
use crate::prelude::*;
use beet_core::prelude::*;

/// `app.bsky.feed.post`: the text, the facets that make parts of it live,
/// and what it replies to or embeds. Keyed by TID, minted on first write.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedPost {
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
	/// The open union of embeds, ie an `app.bsky.embed.external` link card.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub embed: Option<OpenUnion>,
}

impl AtprotoRecord for FeedPost {
	const COLLECTION: Nsid = Nsid::new_static("app.bsky.feed.post");
}

impl FeedPost {
	/// The Bluesky web app, where a post's page lives.
	pub const WEB_APP: &'static str = "https://bsky.app";

	/// The page of the post at `uri` in the Bluesky web app, addressed by did
	/// so a handle change never breaks it, ie
	/// `https://bsky.app/profile/did:plc:../post/3mw72aaeuj22n`.
	pub fn web_url(uri: &AtUri) -> String {
		format!(
			"{}/profile/{}/post/{}",
			Self::WEB_APP,
			uri.did(),
			uri.rkey()
		)
	}

	/// The text cut at its facets, in order: each run with the feature it is,
	/// `None` for plain text. A facet out of bounds, off a char boundary,
	/// overlapping an earlier one or naming no feature stays plain text.
	pub fn segments(&self) -> Vec<(&str, Option<&FacetFeature>)> {
		let mut facets = self.facets.iter().collect::<Vec<_>>();
		facets.sort_by_key(|facet| facet.index.byte_start);
		let mut segments = Vec::new();
		let mut cursor = 0;
		for facet in facets {
			let ByteSlice {
				byte_start,
				byte_end,
			} = facet.index;
			let (Some(feature), Some(live)) =
				(facet.features.first(), self.text.get(byte_start..byte_end))
			else {
				continue;
			};
			if byte_start < cursor || live.is_empty() {
				continue;
			}
			if byte_start > cursor {
				segments.push((&self.text[cursor..byte_start], None));
			}
			segments.push((live, Some(feature)));
			cursor = byte_end;
		}
		if cursor < self.text.len() {
			segments.push((&self.text[cursor..], None));
		}
		segments
	}

	/// The link card the post embeds, `None` for a post embedding anything
	/// else or nothing.
	pub fn external(&self) -> Option<EmbedExternal> {
		self.embed.as_ref().and_then(EmbedExternal::from_union)
	}

	/// A post of `rich` text, written at `created_at`.
	pub fn new(rich: RichText, created_at: Timestamp) -> Self {
		Self {
			text: rich.text,
			facets: rich.facets,
			created_at: created_at.format_iso8601().into(),
			..default()
		}
	}
}

/// `app.bsky.feed.post#replyRef`: where a reply sits, the thread's first
/// post and the one it answers.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplyRef {
	/// The thread's first post.
	pub root: StrongRef,
	/// The post this one answers.
	pub parent: StrongRef,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// The live reply post the record cid fixture is, reading back to the
	/// same body.
	#[cfg(feature = "json")]
	#[beet_core::test]
	fn round_trips_a_pds_post() {
		let json = r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"hi"}"#;
		let value = AtprotoValue::parse_json(json).unwrap();
		let post = value.clone().into_serde::<FeedPost>().unwrap();
		post.reply
			.as_ref()
			.unwrap()
			.root
			.uri
			.rkey()
			.as_str()
			.xpect_eq("3mw5t5lac4k2y");
		AtprotoValue::from_serde(&post)
			.unwrap()
			.into_record(&FeedPost::COLLECTION)
			.unwrap()
			.cid()
			.xpect_eq(value.cid());
	}

	/// The text cuts at each facet, a malformed facet staying plain.
	#[beet_core::test]
	async fn cuts_segments() {
		let rich =
			RichText::new("🤘 #bevy at https://beet.org !", async |_| {
				bevybail!("no mentions")
			})
			.await
			.unwrap();
		let mut post = FeedPost::new(rich, Timestamp::from_millis(1));
		// mid-codepoint, so never live
		post.facets.push(Facet {
			index: ByteSlice {
				byte_start: 1,
				byte_end: 3,
			},
			features: vec![FacetFeature::Tag { tag: "bad".into() }],
		});
		post.segments()
			.into_iter()
			.map(|(text, feature)| (text, feature.is_some()))
			.collect::<Vec<_>>()
			.xpect_eq(vec![
				("🤘 ", false),
				("#bevy", true),
				(" at ", false),
				("https://beet.org", true),
				(" !", false),
			]);
	}

	/// A post with facets and a reply survives the repo's data model, so
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
			FeedPost {
				langs: vec!["en".into()],
				reply: Some(ReplyRef {
					root: parent.clone(),
					parent,
				}),
				..FeedPost::new(rich, Timestamp::from_millis(1))
			}
		};
		let rkey = Rkey::from(Tid::from(Timestamp::from_millis(1)));
		pds.converge([Rkeyed::new(rkey.clone(), post().await)], &default())
			.await
			.unwrap()
			.created
			.len()
			.xpect_eq(1);
		pds.converge([Rkeyed::new(rkey, post().await)], &default())
			.await
			.unwrap()
			.is_noop()
			.xpect_true();
	}
}
