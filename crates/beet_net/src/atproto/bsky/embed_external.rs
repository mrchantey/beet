//! `app.bsky.embed.external`, a link card embedded in a post.
use beet_core::prelude::*;

/// `app.bsky.embed.external`: a link card, the embed a post about a page
/// carries so a reader sees the page's title, description and thumbnail
/// rather than a bare url.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedExternal {
	/// The card.
	pub external: External,
}

impl EmbedExternal {
	/// The def a post's `embed` union names this member by.
	pub const NSID: Nsid = Nsid::new_static("app.bsky.embed.external");

	/// The card for `external`.
	pub fn new(external: External) -> Self { Self { external } }

	/// This card as a member of a post's `embed` union.
	pub fn into_union(self) -> Result<OpenUnion> {
		OpenUnion::from_serde(Self::NSID, &self)
	}

	/// The card `embed` holds, `None` when it is another kind of embed.
	pub fn from_union(embed: &OpenUnion) -> Option<Self> {
		(embed.r#type() == &Self::NSID)
			.then(|| embed.clone().into_serde::<Self>().ok())
			.flatten()
	}
}

/// `app.bsky.embed.external#external`: the linked page as the card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct External {
	/// The linked page, its natural key: an announcement is found again by
	/// the url its card embeds.
	pub uri: SmolStr,
	/// The card's title.
	pub title: String,
	/// The card's description, empty for none.
	pub description: String,
	/// The card's image, under 1MB.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub thumb: Option<BlobRef>,
	/// The atproto records that back the page, ie a standard site document
	/// and its publication, which Bluesky hydrates the card from.
	#[serde(
		default,
		rename = "associatedRefs",
		skip_serializing_if = "Vec::is_empty"
	)]
	pub associated_refs: Vec<StrongRef>,
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;

	/// A post embedding a card survives the repo's data model and reads its
	/// card back, a post with no card reading none.
	#[beet_core::test]
	async fn a_card_round_trips() {
		let card = EmbedExternal::new(External {
			uri: "https://beet.org/blog/full-stack-bevy".into(),
			title: "Full Stack Bevy".into(),
			description: "One language across the stack.".into(),
			thumb: Some(BlobRef::of(b"card", MediaType::Png)),
			associated_refs: vec![StrongRef::new(
				AtUri::parse(
					"at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.document/3mw72aaeuj22n",
				)
				.unwrap(),
				Cid::parse(
					"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
				)
				.unwrap(),
			)],
		});
		let post = FeedPost {
			embed: Some(card.clone().into_union().unwrap()),
			..FeedPost::new(
				RichText::new("see", async |_| bevybail!("none"))
					.await
					.unwrap(),
				Timestamp::from_millis(1),
			)
		};
		AtprotoValue::from_serde(&post)
			.unwrap()
			.into_serde::<FeedPost>()
			.unwrap()
			.external()
			.xpect_eq(Some(card));
		FeedPost::default().external().xpect_none();
	}
}
