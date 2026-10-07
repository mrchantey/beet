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
//! - [`StandardSitePublication`], `site.standard.publication`, built from the
//!   [`StandardSite`] declaration by
//!   [`from_declaration`](StandardSitePublication::from_declaration), with its
//!   [`ThemeBasic`] resolved from the site's [`Theme`]
//! - [`StandardSiteDocument`], `site.standard.document`, built from a listed
//!   page's metadata by [`from_page`](StandardSiteDocument::from_page)
//! - [`StandardSiteContentRenderer`], a format a document's `content` slot
//!   carries, registered by NSID in [`StandardSiteContentRenderers`]
//!
//! # Limits
//!
//! The lexicons cap a `title` at 500 graphemes and 5000 bytes, a
//! `description` at 3000 graphemes and 30000 bytes, each tag at 128
//! graphemes, a contributor's `role` and `displayName` at 100 graphemes, and
//! `icon` and `coverImage` at 1MB each. A builder clamps nothing: a value over
//! a limit is a `check` failure on the page that authored it, since silently
//! truncating a title is worse than refusing to publish it. The tighter rules
//! a page's metadata keeps are `PageMeta`'s.
//!
//! [`Rkeyed`]: beet_core::prelude::Rkeyed
//! [`Theme`]: beet_ui::prelude::Theme
mod content_renderer;
mod document;
mod publication;
mod standard_site;
mod theme;
pub use content_renderer::*;
pub use document::*;
pub use publication::*;
pub use standard_site::*;
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
	pub fn harvest() -> StandardSite {
		StandardSite {
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
