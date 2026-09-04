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
	/// Who wrote the page.
	pub author: Option<SmolStr>,
	/// Excludes the page from production builds when `true`.
	pub draft: bool,
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

impl PageMeta {
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
			.unwrap_or_default()
	}

	/// Every key maps to the field of the same name, through the reflect
	/// coercions a BSX spread resolves through — no hand-written mapping.
	#[beet_core::test]
	fn frontmatter_reads_flat_keys() {
		let meta = parse(
			"title: Getting Started\ndescription: A guide\ndraft: true\norder: 2\nexpanded: true",
			FrontmatterKind::Yaml,
		);
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.description.as_deref().unwrap().xpect_eq("A guide");
		meta.draft.xpect_true();
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
			"slug = \"full-stack-bevy\"\ncreated = \"2025-07-11\"\nauthor = \"Pete Hayman\"\nvideo_url = \"https://youtu.be/7koepBSRoUI\"",
			FrontmatterKind::Toml,
		);
		meta.slug.as_deref().unwrap().xpect_eq("full-stack-bevy");
		meta.created
			.unwrap()
			.format_long_date()
			.xpect_eq("11 July 2025");
		meta.author.as_deref().unwrap().xpect_eq("Pete Hayman");
		meta.video_url
			.as_deref()
			.unwrap()
			.xpect_eq("https://youtu.be/7koepBSRoUI");
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
			declarations.get::<PageMeta>(&registry)?.order
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
