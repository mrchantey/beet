//! Per-page article metadata sourced from markdown frontmatter.
//!
//! [`ArticleMeta`] is inserted on a route's root entity. Its `title`/
//! `description` override the [`PackageConfig`](beet_core::prelude::PackageConfig)
//! defaults in the document `Head`, and its `sidebar` field feeds the per-route
//! [`SidebarInfo`] used by [`SidebarState`](crate::prelude::SidebarState).

use crate::prelude::*;
use beet_core::prelude::*;

/// General metadata common to blog posts, docs pages, etc.
///
/// Built from the markdown [`Frontmatter`](beet_ui::prelude::Frontmatter) via
/// [`from_frontmatter`](Self::from_frontmatter).
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "codegen", derive(ToTokens))]
pub struct ArticleMeta {
	/// Page title; overrides the package title in the document `Head`.
	pub title: Option<String>,
	/// Page description; overrides the package description in the `Head`.
	pub description: Option<String>,
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
	/// Reads the flat keys `title`, `description`, `draft`, plus the sidebar
	/// keys `sidebar_label`, `order`, `expanded`. The sidebar label falls back
	/// to the page title.
	#[cfg(feature = "markdown_parser")]
	pub fn from_frontmatter(
		frontmatter: &beet_ui::prelude::Frontmatter,
	) -> Self {
		Self {
			title: frontmatter.get_str("title").map(String::from),
			description: frontmatter.get_str("description").map(String::from),
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
	fn defaults_when_empty() {
		let frontmatter =
			Frontmatter::parse("", FrontmatterKind::Yaml).unwrap();
		let meta = ArticleMeta::from_frontmatter(&frontmatter);
		meta.xpect_eq(ArticleMeta::default());
		meta.sidebar_label().is_none().xpect_true();
	}
}
