//! Document metadata, declared by markdown frontmatter, a BSX root spread or
//! an Office file's core properties.
//!
//! [`DocMeta`] is inserted on a document's root entity, hoisted there by
//! whichever scan read the document (see
//! [`RootDeclarations`](beet_core::prelude::RootDeclarations)). Its
//! `title`/`description` override the
//! [`PackageConfig`](beet_core::prelude::PackageConfig) defaults in the document
//! [`Head`](crate::prelude::Head), and its `created` and `authors` are read by
//! the entry a generated index renders for it. Where the document serves and
//! how it is listed is [`RouteMeta`](crate::prelude::RouteMeta)'s.
//!
//! It lives here rather than in the router because it is DOCUMENT metadata; the
//! router is one consumer of it, as the head widgets are.
//!
//! # Metadata rules
//!
//! The limits below are not a house style, they are the smallest envelope every
//! consumer of a page's metadata accepts, so one set of frontmatter serves the
//! document head, the feed and a syndicated record with no second pass and no
//! truncation anywhere.
//!
//! - [`description`](DocMeta::description): at most 300 graphemes and 3000
//!   bytes. The tightest limit of the set, and it comes from the furthest
//!   consumer: a post's description becomes the text of the Bluesky post
//!   announcing it, and `app.bsky.feed.post.text` caps at 300 graphemes and
//!   3000 bytes. The same string is the page's meta description, its feed entry
//!   summary and its link card description. Graphemes rather than chars because
//!   an emoji with a skin-tone modifier is one thing a reader sees and should
//!   cost one unit of the budget.
//! - [`title`](DocMeta::title): at most 500 graphemes and 5000 bytes, the
//!   `site.standard.document` limit, which nothing downstream undercuts.
//! - [`tags`](DocMeta::tags): each matching `^[a-z0-9]+(-[a-z0-9]+)*$` and drawn from the
//!   vocabulary its site declares. Lowercase kebab because a tag is a url
//!   segment of its own listing page and a plain string in a syndicated record,
//!   and a closed vocabulary because an open one drifts into synonyms that
//!   split one topic across two names nobody browses.

#[cfg(feature = "serde")]
use crate::prelude::ReflectFrontmatter;
use beet_core::prelude::*;

/// General document metadata: a blog post's, a docs page's, a Word file's.
///
/// Flat by design: every field is a frontmatter key by the same name, so a
/// document's authored surface maps onto the component 1:1 through reflection
/// with no hand-written mapping. It is the default [`FrontmatterType`], so its
/// keys are written unsectioned; a route's own keys sit in a `[RouteMeta]`
/// table (see [`RouteMeta`]).
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Frontmatter))]
pub struct DocMeta {
	/// Document title; overrides the package title in the document `Head`.
	pub title: Option<String>,
	/// Document description; overrides the package description in the `Head`.
	pub description: Option<String>,
	/// The day the page was published.
	///
	/// Authored as a `YYYY-MM-DD` string in either surface, markdown frontmatter
	/// (`created = "2026-08-28"`) or a BSX spread (`{DocMeta{created:".."}}`),
	/// through [`Date`]'s literal parser, so it sorts and formats as a day
	/// rather than as text.
	pub created: Option<Date>,
	/// The day of the last substantive edit, the one a reader judges the
	/// page's freshness by. Authored exactly like [`created`](Self::created),
	/// and unset on a page that has not changed since publication.
	pub updated: Option<Date>,
	/// Who wrote the page, in credit order.
	///
	/// Authored as a list, ie `authors = ["Ada", "Grace"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub authors: Vec<SmolStr>,
	/// The topics the page belongs to, lowercase kebab and drawn from the
	/// vocabulary its site declares (see the module docs' metadata rules).
	///
	/// Authored as a list, ie `tags = ["ecs", "web"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub tags: Vec<SmolStr>,
	/// The self labels a syndicated record of the page carries, effectively
	/// content warnings, normally none.
	///
	/// Authored as a list, ie `labels = ["graphic-media"]`.
	#[cfg_attr(feature = "serde", serde(default))]
	pub labels: Vec<SmolStr>,
	/// The page's thumbnail / social card image.
	pub image_url: Option<Url>,
	/// The page's companion video, eg the YouTube watch url a post embeds.
	pub video_url: Option<Url>,
}

impl DocMeta {
	/// The authors as one line, joined by `, `, or `None` when there are none:
	/// the byline a header, a feed item or a search result shows.
	pub fn byline(&self) -> Option<String> {
		(!self.authors.is_empty()).then(|| self.authors.join(", "))
	}

	/// The date a reader judges the page by: its
	/// [`updated`](Self::updated), else its [`created`](Self::created).
	pub fn last_modified(&self) -> Option<Date> {
		self.updated.or(self.created)
	}

	/// The page's social card image: its own [`image_url`](Self::image_url),
	/// else the thumbnail of the YouTube video it companions.
	///
	/// The derivation is what gets a video post a real link preview with no
	/// frontmatter at all: the poster frame IS the card. `hqdefault` rather
	/// than `maxresdefault`, which 404s for anything uploaded before high-res
	/// thumbnails and so previews as a broken image.
	pub fn social_image_url(&self) -> Option<Url> {
		self.image_url.clone().or_else(|| {
			Self::youtube_id(self.video_url.as_ref()?).map(|id| {
				Url::coerce(format!(
					"https://i.ytimg.com/vi/{id}/hqdefault.jpg"
				))
			})
		})
	}

	/// The video id in a YouTube url: the `7koepBSRoUI` in
	/// `https://youtu.be/..`, `https://www.youtube.com/watch?v=..` or an
	/// already-embed url. `None` for any other url, so a video hosted elsewhere
	/// yields neither an embed nor a derived card.
	///
	/// Read off the url's parts rather than its text, so a share link's `?t=42`
	/// and a watch link's `&list=..` are query params that were never in the
	/// path to begin with.
	pub fn youtube_id(url: &Url) -> Option<&str> {
		let host = url.host()?.trim_start_matches("www.");
		match (host, url.path().as_slice()) {
			("youtu.be", [id]) => Some(id.as_str()),
			("youtube.com", [segment, id]) if segment == "embed" => {
				Some(id.as_str())
			}
			("youtube.com", [segment]) if segment == "watch" => {
				url.get_param("v")
			}
			_ => None,
		}
	}
}

#[cfg(all(test, feature = "bsx"))]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::reflect::TypeRegistry;

	/// Resolve a frontmatter block exactly as a scan does: lower it to root
	/// declarations, then reflect-build this one component out of them.
	fn parse(content: &str, media_type: MediaType) -> DocMeta {
		let mut registry = TypeRegistry::default();
		registry.register::<DocMeta>();
		Frontmatter::parse(content, media_type)
			.unwrap()
			.declarations(&type_ext::short_name::<DocMeta>())
			.get::<DocMeta>(&registry)
			.unwrap()
			.unwrap_or_default()
	}

	/// A `YYYY-MM-DD` string coerces to the day it names, the rule a BSX
	/// spread gets for free and frontmatter used to hand-roll.
	#[beet_core::test]
	fn frontmatter_reads_article_keys() {
		let meta = parse(
			"created = \"2025-07-11\"\nupdated = \"2025-08-01\"\nauthors = [\"Pete Hayman\", \"Ada Lovelace\"]\nvideo_url = \"https://youtu.be/7koepBSRoUI\"\ntags = [\"bevy\", \"web\"]\nlabels = [\"graphic-media\"]",
			MediaType::Toml,
		);
		meta.tags
			.xpect_eq(vec![SmolStr::new("bevy"), SmolStr::new("web")]);
		meta.labels.xpect_eq(vec![SmolStr::new("graphic-media")]);
		meta.created.unwrap().format_long().xpect_eq("11 July 2025");
		// the freshness date a sitemap and a feed read, falling back to `created`
		meta.last_modified()
			.unwrap()
			.to_string()
			.xpect_eq("2025-08-01");
		meta.authors
			.xpect_eq(vec![SmolStr::new("Pete Hayman"), "Ada Lovelace".into()]);
		meta.video_url
			.unwrap()
			.to_string()
			.xpect_eq("https://youtu.be/7koepBSRoUI");
	}

	/// The social card falls back to the companion video's poster frame, so a
	/// video post previews without any `image_url` frontmatter, and a video
	/// hosted elsewhere derives nothing.
	#[beet_core::test]
	fn derives_the_social_card_from_a_video() {
		let card = |video_url: &str| {
			DocMeta {
				video_url: Some(video_url.into()),
				..default()
			}
			.social_image_url()
		};
		let thumbnail = "https://i.ytimg.com/vi/7koepBSRoUI/hqdefault.jpg";
		card("https://youtu.be/7koepBSRoUI")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		// a share url carries a timestamp, the watch url a playlist
		card("https://youtu.be/7koepBSRoUI?t=42")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		card("https://www.youtube.com/watch?v=7koepBSRoUI&list=PL")
			.unwrap()
			.to_string()
			.xpect_eq(thumbnail);
		DocMeta::youtube_id(&Url::coerce(
			"https://www.youtube.com/embed/7koepBSRoUI",
		))
		.unwrap()
		.xpect_eq("7koepBSRoUI");
		card("https://example.com/video.mp4").xpect_none();
		// a bare path is no video at all
		card("/assets/clip.mp4").xpect_none();
		// an explicit image always wins over the derived one
		DocMeta {
			image_url: Some("/assets/card.png".into()),
			video_url: Some("https://youtu.be/7koepBSRoUI".into()),
			..default()
		}
		.social_image_url()
		.unwrap()
		.to_string()
		.xpect_eq("/assets/card.png");
	}

	#[beet_core::test]
	fn defaults_when_empty() {
		let meta = parse("", MediaType::Yaml);
		meta.xpect_eq(DocMeta::default());
	}
}
