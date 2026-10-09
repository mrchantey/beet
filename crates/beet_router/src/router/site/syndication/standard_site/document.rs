//! `site.standard.document`, the record for one published post.
use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// `site.standard.document`: one published post, its metadata, the forms of
/// its body a reader can render without visiting the site, and the
/// announcement its comments hang off. Keyed by TID, minted on first write.
///
/// The `path` is the document's natural key, and joined to its publication's
/// `url` it forms the canonical page, which is also where the full post lives:
/// the record carries the widest-read forms of the body and the canonical page
/// serves the source.
#[derive(Debug, Clone, PartialEq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct StandardSiteDocument {
	/// The publication record the document belongs to, `at://..`, or for a
	/// loose document the url of its site, with no trailing slash; the
	/// record's own view is [`Uri::at_uri`].
	pub site: Uri,
	/// Joined to the publication's url to form the canonical page, with a
	/// leading slash, ie `/full-stack-bevy`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub path: Option<SmolStr>,
	/// The post's title.
	pub title: String,
	/// A brief description, which for a beet post is also its announcement's
	/// text.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	/// When the post was published.
	#[serde(rename = "publishedAt", with = "timestamp_iso8601")]
	pub published_at: Timestamp,
	/// When the post was last substantively edited.
	#[serde(
		default,
		rename = "updatedAt",
		skip_serializing_if = "Option::is_none",
		with = "timestamp_iso8601::option"
	)]
	pub updated_at: Option<Timestamp>,
	/// The post's thumbnail or cover image, under 1MB.
	#[serde(
		default,
		rename = "coverImage",
		skip_serializing_if = "Option::is_none"
	)]
	pub cover_image: Option<BlobRef>,
	/// The whole post as plain text, no markup: what search, a reading time
	/// and any reader that knows no rich format read.
	#[serde(
		default,
		rename = "textContent",
		skip_serializing_if = "Option::is_none"
	)]
	pub text_content: Option<String>,
	/// The post in a rich format a reader may know, an open union, ie
	/// `pub.leaflet.content`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub content: Option<OpenUnion>,
	/// The topics the post belongs to, without a leading `#`.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub tags: Vec<SmolStr>,
	/// Relationships to external resources, a union the lexicon defines no
	/// member of yet.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub links: Option<OpenUnion>,
	/// The post's self labels, effectively content warnings.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub labels: Option<SelfLabels>,
	/// The people credited beyond the repo holding the record.
	#[serde(default, skip_serializing_if = "Vec::is_empty")]
	pub contributors: Vec<Contributor>,
	/// The Bluesky post announcing the document, whose replies are its
	/// comments.
	#[serde(
		default,
		rename = "bskyPostRef",
		skip_serializing_if = "Option::is_none"
	)]
	pub bsky_post_ref: Option<StrongRef>,
}

impl AtprotoRecord for StandardSiteDocument {
	const COLLECTION: Nsid = Nsid::new_static("site.standard.document");
}

impl StandardSiteDocument {
	/// The document the listed page at `path` makes within `publication`,
	/// written to the publication record at `site`, filled from the page's
	/// metadata `meta`: its path beneath the publication, title, description,
	/// dates, tags and labels.
	///
	/// What the publish step derives rather than reads is left empty for it to
	/// fill: the body forms (`text_content`, `content`), the uploaded
	/// `cover_image` (all three [`render`](Self::render)'s), the
	/// `contributors` its authors resolve to and the announcement's
	/// `bsky_post_ref`.
	///
	/// # Errors
	/// Errors when the page is not beneath the publication's path, or lacks
	/// the title or the `created` day the lexicon requires.
	pub fn from_page(
		publication: &StandardSitePub,
		site: AtUri,
		path: &RelPath,
		meta: &PageMeta,
	) -> Result<Self> {
		let relative =
			path.strip_prefix(&publication.path).ok_or_else(|| {
				bevyhow!(
					"page '{path}' is not beneath the publication at '{}'",
					publication.path
				)
			})?;
		Self {
			site: site.into(),
			path: Some(format!("/{relative}").into()),
			title: meta.title.clone().ok_or_else(|| {
				bevyhow!(
					"page '{path}' has no title, which a standard site \
					 document requires"
				)
			})?,
			description: meta.description.clone(),
			published_at: meta
				.created
				.ok_or_else(|| {
					bevyhow!(
						"page '{path}' has no `created` day, which a standard \
						 site document requires as its `publishedAt`"
					)
				})?
				.timestamp(),
			updated_at: meta.updated.map(|updated| updated.timestamp()),
			cover_image: None,
			text_content: None,
			content: None,
			tags: meta.tags.clone(),
			links: None,
			labels: SelfLabels::from_values(meta.labels.iter().cloned()),
			contributors: Vec::new(),
			bsky_post_ref: None,
		}
		.xok()
	}
}

/// `site.standard.document#contributor`: a person credited on a document
/// beyond the repo that holds it, so an author needs no write access to the
/// publication's repo.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub struct Contributor {
	/// The person's account.
	pub did: Did,
	/// What they did, ie `author`.
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub role: Option<SmolStr>,
	/// The name a reader sees.
	#[serde(
		default,
		rename = "displayName",
		skip_serializing_if = "Option::is_none"
	)]
	pub display_name: Option<SmolStr>,
}

impl Contributor {
	/// The role a page's authors are credited with.
	pub const AUTHOR: &'static str = "author";

	/// `account` credited as an author of the document.
	pub fn author(account: &AtprotoAccount) -> Self {
		Self {
			did: account.did().clone(),
			role: Some(SmolStr::new_static(Self::AUTHOR)),
			display_name: Some(account.display_name().clone()),
		}
	}
}

#[cfg(test)]
mod test {
	use super::super::test_fixtures::*;
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;
	use beet_ui::prelude::*;

	/// The listed pages beneath the fixture site's `blog` route, through the
	/// walk the syndication routes share.
	fn blog_pages() -> Vec<SyndicationPage> {
		let mut world = syndication_world(Some("https://beet.org"));
		spawn_syndication_router(&mut world, rsx! {
			<Route path="blog"><RssFeed/></Route>
		});
		let feed = world
			.query::<(Entity, &PathPattern)>()
			.iter(&world)
			.find(|(_, pattern)| {
				pattern.annotated_path() == RelPath::new("blog/rss.xml")
			})
			.unwrap()
			.0;
		world
			.with_state::<SyndicationQuery, _>(|query| query.scope(feed))
			.unwrap()
			.pages
	}

	/// Every dated post of the fixture site, the fields its metadata fills, in
	/// the wire form a repo stores.
	#[beet_core::test]
	fn builds_from_the_fixture_site() {
		blog_pages()
			.iter()
			.filter(|page| page.meta.created.is_some())
			.map(|page| {
				StandardSiteDocument::from_page(
					&harvest(),
					harvest_uri(),
					&page.path,
					&page.meta,
				)
				.unwrap()
			})
			.map(|document| wire(&document))
			.collect::<Vec<_>>()
			.join("\n")
			.xpect_snapshot();
	}

	/// Every field filled, the ones the publish step derives included.
	fn filled() -> StandardSiteDocument {
		let page = SyndicationPage {
			path: RelPath::new("blog/full-stack-bevy"),
			meta: PageMeta {
				title: Some("Full Stack Bevy".into()),
				description: Some("One language across the stack.".into()),
				created: Date::parse("2025-07-11").ok(),
				updated: Date::parse("2025-09-01").ok(),
				tags: vec!["bevy".into(), "web".into()],
				labels: vec!["graphic-media".into()],
				..default()
			},
		};
		let author = AtprotoAccount::new(
			"pete.beet.org",
			Did::parse("did:plc:hnv7bd4gtxrf7iigjo22qukp").unwrap(),
			"Pete Hayman",
		);
		let post = StrongRef::new(
			AtUri::parse(
				"at://did:plc:vgxy56shhs3t3b2mu2avizjf/app.bsky.feed.post/3mw72aaeuj22n",
			)
			.unwrap(),
			Cid::parse(
				"bafyreihzukpzbnzrlnlrg4bweuercgc3r5tz6f37e6qr3n34rneslivaza",
			)
			.unwrap(),
		);
		StandardSiteDocument {
			cover_image: Some(BlobRef::of(COVER, MediaType::Jpeg)),
			text_content: Some("One language.\nOne paradigm.".into()),
			content: Some(
				OpenUnion::new(
					Nsid::new_static("pub.leaflet.content"),
					value!({ "pages": [] }),
				)
				.unwrap(),
			),
			contributors: vec![Contributor::author(&author)],
			bsky_post_ref: Some(post),
			..StandardSiteDocument::from_page(
				&harvest(),
				harvest_uri(),
				&page.path,
				&page.meta,
			)
			.unwrap()
		}
	}

	/// The bytes of the fixture's cover image.
	const COVER: &[u8] = b"cover";

	#[beet_core::test]
	fn serializes_the_lexicon() { wire(&filled()).xpect_snapshot(); }

	#[beet_core::test]
	async fn converges() {
		converges_once(&[(COVER, MediaType::Jpeg)], filled()).await;
	}

	/// A page outside the publication, or missing what the lexicon requires,
	/// is refused rather than published as a guess.
	#[beet_core::test]
	fn refuses_an_incomplete_page() {
		let page = |path: &str, meta: PageMeta| SyndicationPage {
			path: RelPath::new(path),
			meta,
		};
		let dated = PageMeta {
			title: Some("Post".into()),
			created: Date::parse("2025-07-11").ok(),
			..default()
		};
		let build = |page: SyndicationPage| {
			StandardSiteDocument::from_page(
				&harvest(),
				harvest_uri(),
				&page.path,
				&page.meta,
			)
			.unwrap_err()
			.to_string()
		};
		build(page("docs/post", dated.clone())).xpect_contains("not beneath");
		build(page("blog/post", PageMeta {
			title: None,
			..dated.clone()
		}))
		.xpect_contains("no title");
		build(page("blog/post", PageMeta {
			created: None,
			..dated
		}))
		.xpect_contains("created");
	}

	/// Standard.site's own document as its PDS answered on 2026-10-06, a
	/// field beet does not know (`canonicalUrl`) ignored.
	#[beet_core::test]
	fn reads_a_foreign_document() {
		let json = r#"{"path":"/docs/lexicons/recommend","site":"at://did:plc:re3ebnp5v7ffagz6rb6xfei4/site.standard.publication/3me5vykp6lf2y","$type":"site.standard.document","title":"Recommend Lexicon","coverImage":{"ref":{"$link":"bafkreibyshzq4yiashq67ajipj3eo6n2dybbvsdmzspox7mpgoxtfimoeq"},"size":113475,"$type":"blob","mimeType":"image/png"},"description":"Schema reference for document recommends, used to declare that a user endorses or recommends a document.","publishedAt":"2026-05-19T00:00:00.000Z","canonicalUrl":"https://standard.site/docs/lexicons/recommend"}"#;
		let document =
			AtprotoValue::from_json(serde_json::from_str(json).unwrap())
				.unwrap()
				.into_serde::<StandardSiteDocument>()
				.unwrap();
		document
			.site
			.at_uri()
			.unwrap()
			.rkey()
			.as_str()
			.xpect_eq("3me5vykp6lf2y");
		document
			.published_at
			.xpect_eq(Date::parse("2026-05-19").unwrap().timestamp());
		document.cover_image.unwrap().size.xpect_eq(113475);
	}
}
