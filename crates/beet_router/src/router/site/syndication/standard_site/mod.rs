//! A site's posts as [standard.site](https://standard.site) records: the
//! publication describing the site, and one standard site document per post.
//!
//! Standard.site is the community lexicon set for long-form publishing on
//! atproto. Bluesky renders an enhanced link card for a url whose standard site
//! document verifies, and Leaflet and the other standard.site readers list,
//! subscribe to and render the records. The words (publication, standard site
//! document, announcement, derived file) are the glossary's; the protocol's
//! own (rkey, TID, strong ref, blob) are `beet_core::atproto`'s.
//!
//! Each record type is its lexicon and nothing else, named for its def and
//! serialized with the lexicon's exact wire names: the rkey a record lives at
//! travels beside it in an [`Rkeyed`], never in the body.
//!
//! - [`StandardSitePub`], `<StandardSitePub/>`, the declaration: the
//!   publication's data, its account, and the derived [`StandardSiteIndex`]
//!   remembering where each record went, whose module docs say what the
//!   served site makes of it (the well-known route, the head's `<link>`s)
//! - [`StandardSitePublication`], `site.standard.publication`, built from the
//!   declaration by
//!   [`from_declaration`](StandardSitePublication::from_declaration), with its
//!   [`ThemeBasic`] resolved from the site's [`Theme`]
//! - [`StandardSiteDocument`], `site.standard.document`, built from a listed
//!   page's metadata and rendered off its live page (the `publish` module:
//!   the body forms, the cover, the blob upload)
//! - [`StandardSiteComments`], an article's comments, the replies to its
//!   announcement read live from the AppView with the `atproto` feature, and
//!   [`StandardSiteReplyLink`], the link to reply
//! - the [`StandardSiteContentFormats`] registry a publication's `content`
//!   NSID resolves through, and with the `leaflet` feature `LeafletContent`,
//!   `pub.leaflet.content`, the rich `content` format, rendered by the
//!   `LeafletRenderer` target
//!
//! # Limits
//!
//! The lexicons cap a `title` at 500 graphemes and 5000 bytes, a
//! `description` at 3000 graphemes and 30000 bytes, each tag at 128
//! graphemes, a contributor's `role` and `displayName` at 100 graphemes, and
//! `icon` and `coverImage` at 1MB each. A builder clamps nothing: a value over
//! a limit is a `check` failure on the page that authored it, since silently
//! truncating a title is worse than refusing to publish it. The tighter rules
//! a page's metadata keeps are `DocMeta`'s.
//!
//! [`Rkeyed`]: beet_core::prelude::Rkeyed
//! [`Theme`]: beet_ui::prelude::Theme
mod comments;
mod content_format;
mod document;
#[cfg(feature = "leaflet")]
mod leaflet;
mod publication;
#[cfg(feature = "json")]
mod publish;
mod standard_site_index;
mod standard_site_pub;
mod theme;
pub use comments::*;
pub use content_format::*;
pub use document::*;
#[cfg(feature = "leaflet")]
pub use leaflet::*;
pub use publication::*;
pub use standard_site_index::*;
pub use standard_site_pub::*;
pub use theme::*;

/// What every standard site test serializes against: the fixture
/// declaration, the address its publication was written at, and the wire
/// form a record is stored in.
#[cfg(test)]
pub(crate) mod test_fixtures {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The blog of the syndication fixture site as a publication.
	pub fn harvest() -> StandardSitePub {
		StandardSitePub {
			path: RelPath::new("blog"),
			name: "The Full Moon Harvest".into(),
			description: Some("Monthly news from the beet garden".into()),
			..default()
		}
	}

	/// The address the fixture publication was written at.
	pub fn harvest_uri() -> AtUri {
		AtUri::parse(
			"at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.publication/3mw72aaeuj22n",
		)
		.unwrap()
	}

	/// The fixture publication's index: the publication, and the one post
	/// published, `blog/full-stack-bevy`.
	pub fn harvest_index() -> StandardSiteIndex {
		let cid = || {
			Cid::parse(
				"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
			)
			.unwrap()
		};
		StandardSiteIndex {
			publication: StrongRef::new(harvest_uri(), cid()),
			documents: vec![StandardSiteIndexDocument {
				path: "/full-stack-bevy".into(),
				uri: AtUri::parse(
					"at://did:plc:vgxy56shhs3t3b2mu2avizjf/site.standard.document/3mw72aaeuj22o",
				)
				.unwrap(),
				cid: cid(),
				bsky_post: None,
			}],
		}
	}

	/// Write `index` to `store` at the fixture publication's index path, as
	/// `publish` writes it.
	pub async fn write_index(store: &BlobStore, index: &StandardSiteIndex) {
		let body = EntityBody::default()
			.with(index)
			.unwrap()
			.with(&Provenance::new([]))
			.unwrap();
		store
			.insert(&harvest().index, MediaType::Json.serialize(&body).unwrap())
			.await
			.unwrap();
	}

	/// Upload `blobs`, then write `record` twice through the converge: one
	/// create, then nothing, so the record reads back through its own type
	/// unchanged.
	pub async fn converges_once<T: AtprotoRecord + Clone>(
		blobs: &[(&'static [u8], MediaType)],
		record: T,
	) {
		let pds = Pds::temp();
		for (bytes, media_type) in blobs {
			pds.upload_blob(*bytes, media_type.clone()).await.unwrap();
		}
		let rkey = Rkey::from(Tid::from(Timestamp::from_millis(1)));
		pds.converge([Rkeyed::new(rkey.clone(), record.clone())], &default())
			.await
			.unwrap()
			.created
			.len()
			.xpect_eq(1);
		pds.converge([Rkeyed::new(rkey, record)], &default())
			.await
			.unwrap()
			.is_noop()
			.xpect_true();
	}

	/// `record` exactly as a repo stores it: in the data model, its `$type`
	/// written as its collection, as indented json.
	pub fn wire<T: AtprotoRecord>(record: &T) -> String {
		AtprotoValue::from_serde(record)
			.unwrap()
			.into_record(&T::COLLECTION)
			.unwrap()
			.to_json()
			.xmap(|json| serde_json::to_string_pretty(&json))
			.unwrap()
	}
}
