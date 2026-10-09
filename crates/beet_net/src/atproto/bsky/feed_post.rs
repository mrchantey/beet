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
	#[beet_core::test]
	fn round_trips_a_pds_post() {
		let json = r#"{"$type":"app.bsky.feed.post","createdAt":"2026-09-23T15:22:16.682Z","langs":["en"],"reply":{"parent":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"},"root":{"cid":"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza","uri":"at://did:plc:y2aci3l7tvrs3vuoz6tou2eb/app.bsky.feed.post/3mw5t5lac4k2y"}},"text":"hi"}"#;
		let value =
			AtprotoValue::from_json(serde_json::from_str(json).unwrap())
				.unwrap();
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
