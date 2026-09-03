//! Per-page article metadata sourced from markdown frontmatter.
//!
//! [`ArticleMeta`] is inserted on a route's root entity. Its `title`/
//! `description` override the [`PackageConfig`](beet_core::prelude::PackageConfig)
//! defaults in the document `Head`, its `sidebar` field feeds the per-route
//! [`SidebarInfo`] used by [`SidebarState`](crate::prelude::SidebarState), and
//! its `slug`/`created`/`author` drive the url a page serves at and the entry
//! [`RouteIndex`](crate::prelude::RouteIndex) renders for it.

use crate::prelude::*;
use beet_core::prelude::*;

/// General metadata common to blog posts, docs pages, etc.
///
/// Built from the markdown [`Frontmatter`](beet_ui::prelude::Frontmatter) via
/// [`from_frontmatter`](Self::from_frontmatter).
///
/// Two of its fields are *url* metadata rather than page metadata, applied by
/// whichever scan discovered the file (see [`apply_slug`](Self::apply_slug) and
/// [`with_file_defaults`](Self::with_file_defaults)): a `slug` renames the
/// route's last segment, and a numbered filename orders it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codegen", derive(ToTokens))]
pub struct ArticleMeta {
	/// Page title; overrides the package title in the document `Head`.
	pub title: Option<String>,
	/// Page description; overrides the package description in the `Head`.
	pub description: Option<String>,
	/// The single url segment this page serves at, replacing the one its
	/// filename implies, so a file may be numbered for reading order while its
	/// url stays a stable name: `blog/1-full-stack-bevy.md` declaring
	/// `slug = "full-stack-bevy"` serves at `blog/full-stack-bevy`.
	pub slug: Option<SmolStr>,
	/// Publication date as an ISO `YYYY-MM-DD` string, eg `2026-08-28`.
	///
	/// A string, not a date type: [`Frontmatter`](beet_ui::prelude::Frontmatter)
	/// carries flat scalars only. ISO order is lexical order, so it sorts as-is.
	pub created: Option<SmolStr>,
	/// Who wrote the page.
	pub author: Option<SmolStr>,
	/// Excludes the page from production builds when `true`.
	pub draft: bool,
	/// Per-route sidebar override (label/order/expanded).
	pub sidebar: SidebarInfo,
}

impl ArticleMeta {
	/// The sidebar label: explicit `sidebar.label`, else the page `title`.
	pub fn sidebar_label(&self) -> Option<&str> {
		self.sidebar.label.as_deref().or(self.title.as_deref())
	}

	/// The route's [`SidebarInfo`]: its `sidebar` override with the label
	/// resolved via [`sidebar_label`](Self::sidebar_label).
	pub fn sidebar_info(&self) -> SidebarInfo {
		let mut info = self.sidebar.clone();
		info.label = self.sidebar_label().map(String::from);
		info
	}

	/// The url `route_path` serves at: its final segment replaced by this
	/// article's [`slug`](Self::slug), if it declares one.
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

	/// Fill the defaults a content file's NAME implies: a leading `<number>-` on
	/// the filename sets the sidebar [`order`](SidebarInfo::order) unless the
	/// frontmatter declared one.
	///
	/// So a numbered directory keeps its reading order in the nav and in a
	/// [`RouteIndex`](crate::prelude::RouteIndex) even once a
	/// [`slug`](Self::slug) has taken the number out of the url, and `10-` sorts
	/// after `2-` where a string compare of the url would not.
	pub fn with_file_defaults(mut self, file: &SmolPath) -> Self {
		self.sidebar.order = self.sidebar.order.or_else(|| {
			file.file_stem()?
				.split_once('-')
				.and_then(|(prefix, _)| prefix.parse().ok())
		});
		self
	}

	/// Parse a markdown source's leading frontmatter, if any.
	///
	/// The scan-time entry point shared by [`RoutesDir`](crate::prelude::RoutesDir)
	/// discovery and the codegen collection scan, so both route kinds carry
	/// eager metadata.
	#[cfg(feature = "markdown_parser")]
	pub fn from_markdown(source: &str) -> Option<Self> {
		beet_ui::prelude::Frontmatter::extract(source)
			.ok()
			.flatten()
			.map(|frontmatter| Self::from_frontmatter(&frontmatter))
	}

	/// Build from parsed markdown [`Frontmatter`](beet_ui::prelude::Frontmatter).
	///
	/// Reads the flat keys `title`, `description`, `slug`, `created`, `author`,
	/// `draft`, plus the sidebar keys `sidebar_label`, `order`, `expanded`. The
	/// sidebar label falls back to the page title.
	#[cfg(feature = "markdown_parser")]
	pub fn from_frontmatter(
		frontmatter: &beet_ui::prelude::Frontmatter,
	) -> Self {
		Self {
			title: frontmatter.get_str("title").map(String::from),
			description: frontmatter.get_str("description").map(String::from),
			slug: frontmatter.get_str("slug").map(SmolStr::new),
			created: frontmatter.get_str("created").map(SmolStr::new),
			author: frontmatter.get_str("author").map(SmolStr::new),
			draft: frontmatter.get_bool("draft").unwrap_or(false),
			sidebar: SidebarInfo {
				label: frontmatter.get_str("sidebar_label").map(String::from),
				order: frontmatter.get_uint("order").map(|order| order as u32),
				expanded: frontmatter.get_bool("expanded"),
			},
		}
	}
}

#[cfg(all(test, feature = "markdown_parser"))]
mod test {
	use super::*;
	use beet_ui::prelude::*;

	#[beet_core::test]
	fn from_frontmatter_reads_flat_keys() {
		let frontmatter = Frontmatter::parse(
			"title: Getting Started\ndescription: A guide\ndraft: true\norder: 2\nexpanded: true",
			FrontmatterKind::Yaml,
		)
		.unwrap();
		let meta = ArticleMeta::from_frontmatter(&frontmatter);
		meta.title.as_deref().unwrap().xpect_eq("Getting Started");
		meta.description.as_deref().unwrap().xpect_eq("A guide");
		meta.draft.xpect_true();
		meta.sidebar.order.unwrap().xpect_eq(2);
		meta.sidebar.expanded.unwrap().xpect_true();
		// no explicit sidebar_label, so the label falls back to the title
		meta.sidebar_label().unwrap().xpect_eq("Getting Started");
	}

	#[beet_core::test]
	fn from_frontmatter_reads_article_keys() {
		let frontmatter = Frontmatter::parse(
			"slug = \"full-stack-bevy\"\ncreated = \"2025-07-11\"\nauthor = \"Pete Hayman\"",
			FrontmatterKind::Toml,
		)
		.unwrap();
		let meta = ArticleMeta::from_frontmatter(&frontmatter);
		meta.slug.as_deref().unwrap().xpect_eq("full-stack-bevy");
		meta.created.as_deref().unwrap().xpect_eq("2025-07-11");
		meta.author.as_deref().unwrap().xpect_eq("Pete Hayman");
	}

	#[beet_core::test]
	fn defaults_when_empty() {
		let frontmatter =
			Frontmatter::parse("", FrontmatterKind::Yaml).unwrap();
		let meta = ArticleMeta::from_frontmatter(&frontmatter);
		meta.xpect_eq(ArticleMeta::default());
		meta.sidebar_label().is_none().xpect_true();
	}

	#[beet_core::test]
	fn apply_slug_renames_last_segment() {
		let meta = ArticleMeta {
			slug: Some("full-stack-bevy".into()),
			..default()
		};
		meta.apply_slug(&SmolPath::new("blog/1-full-stack-bevy"))
			.unwrap()
			.xpect_eq(SmolPath::new("blog/full-stack-bevy"));
		// no slug declared, the filename-derived path stands
		ArticleMeta::default()
			.apply_slug(&SmolPath::new("blog/post-1"))
			.unwrap()
			.xpect_eq(SmolPath::new("blog/post-1"));
		// a nested slug is a loud error, never a silent re-parent
		ArticleMeta {
			slug: Some("blog/nested".into()),
			..default()
		}
		.apply_slug(&SmolPath::new("blog/post-1"))
		.unwrap_err()
		.to_string()
		.xpect_contains("single path segment");
	}

	#[beet_core::test]
	fn with_file_defaults_orders_by_number() {
		ArticleMeta::default()
			.with_file_defaults(&SmolPath::new("blog/10-later.md"))
			.sidebar
			.order
			.unwrap()
			.xpect_eq(10);
		// an explicit frontmatter order wins over the filename
		ArticleMeta {
			sidebar: SidebarInfo {
				order: Some(3),
				..default()
			},
			..default()
		}
		.with_file_defaults(&SmolPath::new("blog/10-later.md"))
		.sidebar
		.order
		.unwrap()
		.xpect_eq(3);
		// an unnumbered filename stays unordered
		ArticleMeta::default()
			.with_file_defaults(&SmolPath::new("docs/intro.md"))
			.sidebar
			.order
			.is_none()
			.xpect_true();
	}
}
