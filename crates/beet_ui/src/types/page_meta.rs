//! Per-page document metadata, declared by markdown frontmatter or a BSX root
//! spread.
//!
//! [`PageMeta`] is inserted on a document's root entity, hoisted there by
//! whichever scan read the document (see
//! [`RootDeclarations`](beet_core::prelude::RootDeclarations)). Its
//! `title`/`description` override the
//! [`PackageConfig`](beet_core::prelude::PackageConfig) defaults in the document
//! [`Head`](crate::prelude::Head), and its `sidebar_label`/`order`/`expanded`,
//! `slug`, `created` and `author` are read by the router: the url a page serves
//! at, its place in the nav, and the entry a generated index renders for it.
//!
//! It lives here rather than in the router because it is DOCUMENT metadata; the
//! router is one consumer of it, as the head widgets are.

use beet_core::prelude::*;

/// General metadata common to blog posts, docs pages, etc.
///
/// Flat by design: every field is a frontmatter key by the same name, so a
/// document's authored surface maps onto the component 1:1 through reflection
/// with no hand-written mapping.
///
/// Two of its fields are *url* metadata rather than page metadata, applied by
/// whichever scan discovered the file (see [`apply_slug`](Self::apply_slug) and
/// [`declare_file_defaults`](Self::declare_file_defaults)): a `slug` renames the
/// route's last segment, and a numbered filename orders it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PageMeta {
	/// Page title; overrides the package title in the document `Head`.
	pub title: Option<String>,
	/// Page description; overrides the package description in the `Head`.
	pub description: Option<String>,
	/// The single url segment this page serves at, replacing the one its
	/// filename implies, so a file may be numbered for reading order while its
	/// url stays a stable name: `blog/1-full-stack-bevy.md` declaring
	/// `slug = "full-stack-bevy"` serves at `blog/full-stack-bevy`.
	pub slug: Option<SmolStr>,
	/// Publication date, ie midnight UTC on the day the page was published.
	///
	/// Authored as a `YYYY-MM-DD` string in either surface — markdown frontmatter
	/// (`created = "2026-08-28"`) or a BSX spread (`{PageMeta{created:".."}}`,
	/// coerced by the reflect string-to-[`Timestamp`] rule) — and parsed to an
	/// instant here, so it sorts and formats as a date rather than as text.
	pub created: Option<Timestamp>,
	/// The last substantive edit, ie the date a reader should judge the page's
	/// freshness by. Authored exactly like [`created`](Self::created), and
	/// unset on a page that has not changed since publication.
	pub updated: Option<Timestamp>,
	/// Who wrote the page.
	pub author: Option<SmolStr>,
	/// Who the page is for: everyone, whoever holds the link, or nobody yet.
	///
	/// Authored by variant name, ie `visibility = "Unlisted"`.
	pub visibility: PageVisibility,
	/// The page's thumbnail / social card image.
	pub image_url: Option<SmolStr>,
	/// The page's companion video, eg the YouTube watch url a post embeds.
	pub video_url: Option<SmolStr>,
	/// Sidebar label override. Defaults to the page [`title`](Self::title).
	pub sidebar_label: Option<String>,
	/// Sort order within siblings, in the nav and in a generated page index.
	/// Lower values come first.
	pub order: Option<u32>,
	/// Force the nav branch open (`Some(true)`) or closed (`Some(false)`);
	/// `None` auto-expands when the current path is a descendant.
	pub expanded: Option<bool>,
}

/// Who a page is for, ie how far it travels: the one knob every listing,
/// crawler and export gate reads.
///
/// Three states rather than a `draft` flag because "not finished" and "not
/// advertised" are different intents with different handling: a draft must
/// never reach production at all, while an unlisted page must serve to whoever
/// holds its link and appear in no index.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PageVisibility {
	/// Fully public: served, exported, and listed everywhere.
	#[default]
	Public,
	/// Served and exported, but excluded from the sitemap, the feeds and the
	/// search index, and marked `noindex` for crawlers.
	Unlisted,
	/// Excluded from production entirely, whether exported or live-served.
	Draft,
}

impl PageMeta {
	/// Whether the page is listed: in a sitemap, a feed, a search index, and to
	/// a crawler. False for both [`Unlisted`](PageVisibility::Unlisted) and
	/// [`Draft`](PageVisibility::Draft) pages.
	pub fn is_listed(&self) -> bool {
		self.visibility == PageVisibility::Public
	}

	/// Whether the page is unfinished, ie must not reach production.
	pub fn is_draft(&self) -> bool { self.visibility == PageVisibility::Draft }

	/// The date a reader judges the page by: its
	/// [`updated`](Self::updated), else its [`created`](Self::created).
	pub fn last_modified(&self) -> Option<Timestamp> {
		self.updated.or(self.created)
	}

	/// The page's social card image: its own [`image_url`](Self::image_url),
	/// else the thumbnail of the YouTube video it companions.
	///
	/// The derivation is what gets a video post a real link preview with no
	/// frontmatter at all: the poster frame IS the card. `hqdefault` rather
	/// than `maxresdefault`, which 404s for anything uploaded before high-res
	/// thumbnails and so previews as a broken image.
	pub fn social_image_url(&self) -> Option<SmolStr> {
		self.image_url.clone().or_else(|| {
			Self::youtube_id(self.video_url.as_deref()?).map(|id| {
				SmolStr::new(format!(
					"https://i.ytimg.com/vi/{id}/hqdefault.jpg"
				))
			})
		})
	}

	/// The video id in a YouTube url: the `7koepBSRoUI` in
	/// `https://youtu.be/..`, `https://www.youtube.com/watch?v=..` or an
	/// already-embed url. `None` for any other url, so a video hosted elsewhere
	/// yields neither an embed nor a derived card.
	pub fn youtube_id(url: &str) -> Option<&str> {
		url.split_once("youtu.be/")
			.or_else(|| url.split_once("youtube.com/embed/"))
			.or_else(|| url.split_once("watch?v="))
			.map(|(_, id)| id)?
			.split(['?', '&', '#'])
			.next()
			.filter(|id| !id.is_empty())
	}

	/// The sidebar label: explicit [`sidebar_label`](Self::sidebar_label), else
	/// the page [`title`](Self::title).
	pub fn sidebar_label(&self) -> Option<&str> {
		self.sidebar_label.as_deref().or(self.title.as_deref())
	}

	/// The url `route_path` serves at: its final segment replaced by this
	/// page's [`slug`](Self::slug), if it declares one.
	///
	/// Applied by the discovering scan, where the frontmatter is in hand, so the
	/// filename-to-path derivation itself stays a pure function of the filename.
	///
	/// # Errors
	/// Errors when the slug contains a `/`. A slug names ONE segment; a slug
	/// quietly re-parenting a page is worse than a loud entry.
	pub fn apply_slug(&self, route_path: &SmolPath) -> Result<SmolPath> {
		let Some(slug) = &self.slug else {
			return Ok(route_path.clone());
		};
		if slug.contains('/') {
			bevybail!(
				"Invalid slug '{slug}' on route '{route_path}': a slug is a single path segment, so it may not contain '/'"
			);
		}
		let mut segments = route_path.segments();
		match segments.last_mut() {
			Some(last) => *last = slug,
			None => segments.push(slug),
		}
		SmolPath::from_segments(&segments).xok()
	}

	/// Declare onto `declarations` the defaults a content file's NAME implies: a
	/// leading `<number>-` on the filename sets [`order`](Self::order), unless the
	/// document declared one.
	///
	/// A patch of the LITERAL rather than of a built value, so the discovery scan
	/// and the codegen emit share it: an emitted route carries no filename to
	/// derive the order from at spawn.
	///
	/// So a numbered directory keeps its reading order in the nav and in a
	/// generated page index even once a [`slug`](Self::slug) has taken the number
	/// out of the url, and `10-` sorts after `2-` where a string compare of the
	/// url would not.
	#[cfg(feature = "bsx")]
	pub fn declare_file_defaults(
		declarations: &mut RootDeclarations,
		file: &SmolPath,
	) {
		let Some(order) = file
			.file_stem()
			.and_then(|stem| stem.split_once('-'))
			.and_then(|(prefix, _)| prefix.parse::<u64>().ok())
		else {
			return;
		};
		declarations.declare_default(
			&type_ext::short_name::<Self>(),
			"order",
			DataLiteral::Scalar(Value::Uint(order)),
		);
	}
}

#[cfg(all(test, feature = "bsx"))]
mod test {
	use super::*;
	use crate::prelude::*;
	use bevy::reflect::TypeRegistry;

	/// Resolve a frontmatter block exactly as a scan does: lower it to root
	/// declarations, then reflect-build this one component out of them.
	fn parse(content: &str, kind: FrontmatterKind) -> PageMeta {
		let mut registry = TypeRegistry::default();
		registry.register::<PageMeta>();
		Frontmatter::parse(content, kind)
			.unwrap()
			.declarations(&type_ext::short_name::<PageMeta>())
			.get::<PageMeta>(&registry)
			.unwrap()
			.unwrap_or_default()
	}

	/// Every key maps to the field of the same name, through the reflect
	/// coercions a BSX spread resolves through — no hand-written mapping.
	#[beet_core::test]
	fn frontmatter_reads_flat_keys() {
		let meta = parse(
			"title: Getting Started\ndescription: A guide\nvisibility: Draft\norder: 2\nexpanded: true",
			FrontmatterKind::Yaml,
		);
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.description.as_deref().unwrap().xpect_eq("A guide");
		meta.is_draft().xpect_true();
		meta.is_listed().xpect_false();
		meta.order.unwrap().xpect_eq(2);
		meta.expanded.unwrap().xpect_true();
		// no explicit sidebar_label, so the label falls back to the title
		meta.sidebar_label().unwrap().xpect_eq("Getting Started");
	}

	/// A `YYYY-MM-DD` string coerces to the instant it names, the rule a BSX
	/// spread gets for free and frontmatter used to hand-roll.
	#[beet_core::test]
	fn frontmatter_reads_article_keys() {
		let meta = parse(
			"slug = \"full-stack-bevy\"\ncreated = \"2025-07-11\"\nupdated = \"2025-08-01\"\nauthor = \"Pete Hayman\"\nvideo_url = \"https://youtu.be/7koepBSRoUI\"",
			FrontmatterKind::Toml,
		);
		meta.slug.as_deref().unwrap().xpect_eq("full-stack-bevy");
		meta.created
			.unwrap()
			.format_long_date()
			.xpect_eq("11 July 2025");
		// the freshness date a sitemap and a feed read, falling back to `created`
		meta.last_modified()
			.unwrap()
			.format_date()
			.xpect_eq("2025-08-01");
		meta.author.as_deref().unwrap().xpect_eq("Pete Hayman");
		meta.video_url
			.as_deref()
			.unwrap()
			.xpect_eq("https://youtu.be/7koepBSRoUI");
	}

	/// The social card falls back to the companion video's poster frame, so a
	/// video post previews without any `image_url` frontmatter, and a video
	/// hosted elsewhere derives nothing.
	#[beet_core::test]
	fn derives_the_social_card_from_a_video() {
		let card = |video_url: &str| {
			PageMeta {
				video_url: Some(video_url.into()),
				..default()
			}
			.social_image_url()
		};
		card("https://youtu.be/7koepBSRoUI")
			.unwrap()
			.xpect_eq(SmolStr::new(
				"https://i.ytimg.com/vi/7koepBSRoUI/hqdefault.jpg",
			));
		// a share url carries a timestamp, the watch url a playlist
		card("https://youtu.be/7koepBSRoUI?t=42").unwrap().xpect_eq(
			SmolStr::new("https://i.ytimg.com/vi/7koepBSRoUI/hqdefault.jpg"),
		);
		card("https://www.youtube.com/watch?v=7koepBSRoUI&list=PL")
			.unwrap()
			.xpect_eq(SmolStr::new(
				"https://i.ytimg.com/vi/7koepBSRoUI/hqdefault.jpg",
			));
		PageMeta::youtube_id("https://www.youtube.com/embed/7koepBSRoUI")
			.unwrap()
			.xpect_eq("7koepBSRoUI");
		card("https://example.com/video.mp4").xpect_none();
		// an explicit image always wins over the derived one
		PageMeta {
			image_url: Some("/assets/card.png".into()),
			video_url: Some("https://youtu.be/7koepBSRoUI".into()),
			..default()
		}
		.social_image_url()
		.unwrap()
		.xpect_eq(SmolStr::new("/assets/card.png"));
	}

	#[beet_core::test]
	fn defaults_when_empty() {
		let meta = parse("", FrontmatterKind::Yaml);
		meta.xpect_eq(PageMeta::default());
		meta.sidebar_label().is_none().xpect_true();
	}

	#[beet_core::test]
	fn apply_slug_renames_last_segment() {
		let meta = PageMeta {
			slug: Some("full-stack-bevy".into()),
			..default()
		};
		meta.apply_slug(&SmolPath::new("blog/1-full-stack-bevy"))
			.unwrap()
			.xpect_eq(SmolPath::new("blog/full-stack-bevy"));
		// no slug declared, the filename-derived path stands
		PageMeta::default()
			.apply_slug(&SmolPath::new("blog/post-1"))
			.unwrap()
			.xpect_eq(SmolPath::new("blog/post-1"));
		// a nested slug is a loud error, never a silent re-parent
		PageMeta {
			slug: Some("blog/nested".into()),
			..default()
		}
		.apply_slug(&SmolPath::new("blog/post-1"))
		.unwrap_err()
		.to_string()
		.xpect_contains("single path segment");
	}

	#[beet_core::test]
	fn file_defaults_order_by_number() {
		let mut registry = TypeRegistry::default();
		registry.register::<PageMeta>();
		let order = |file: &str, frontmatter: &str| {
			let mut declarations =
				Frontmatter::parse(frontmatter, FrontmatterKind::Toml)
					.unwrap()
					.declarations(&type_ext::short_name::<PageMeta>());
			PageMeta::declare_file_defaults(
				&mut declarations,
				&SmolPath::new(file),
			);
			declarations.get::<PageMeta>(&registry).unwrap()?.order
		};
		order("blog/10-later.md", "title = \"Later\"")
			.unwrap()
			.xpect_eq(10);
		// an explicit frontmatter order wins over the filename
		order("blog/10-later.md", "order = 3").unwrap().xpect_eq(3);
		// an unnumbered filename stays unordered
		order("docs/intro.md", "title = \"Intro\"")
			.is_none()
			.xpect_true();
		// a document declaring nothing gets no metadata invented for it
		order("blog/10-later.md", "").is_none().xpect_true();
	}
}
