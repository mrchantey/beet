//! Route metadata: where a document serves, its place in the nav and how far
//! it travels, declared by a markdown frontmatter `[RouteMeta]` table or a BSX
//! root spread, beside the document's own [`DocMeta`].
//!
//! ```toml
//! title = "Getting Started"
//!
//! [RouteMeta]
//! order = 2
//! slug = "getting-started"
//! ```

#[cfg(feature = "serde")]
use crate::prelude::ReflectFrontmatter;
use crate::prelude::*;
use beet_core::prelude::*;

/// The metadata of a document served at a path: its url segment, its order
/// and label in the nav, and its [`RouteVisibility`].
///
/// Two of its fields are applied by whichever scan discovered the file (see
/// [`apply_slug`](Self::apply_slug) and
/// [`declare_file_defaults`](Self::declare_file_defaults)): a `slug` renames
/// the route's last segment, and a numbered filename orders it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Component, Reflect)]
#[reflect(Component, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", reflect(Frontmatter))]
pub struct RouteMeta {
	/// The single url segment this page serves at, replacing the one its
	/// filename implies, so a file may be numbered for reading order while its
	/// url stays a stable name: `blog/1-full-stack-bevy.md` declaring
	/// `slug = "full-stack-bevy"` serves at `blog/full-stack-bevy`.
	pub slug: Option<SmolStr>,
	/// Who the page is for: everyone, whoever holds the link, or nobody yet.
	///
	/// Authored by variant name, ie `visibility = "unlisted"` (the match is
	/// case-insensitive, so the Rust spelling works too).
	pub visibility: RouteVisibility,
	/// Sidebar label override. Defaults to the document [`title`](DocMeta::title).
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
pub enum RouteVisibility {
	/// Fully public: served, exported, and listed everywhere.
	#[default]
	Public,
	/// Served and exported, but excluded from the sitemap, the feeds and the
	/// search index, and marked `noindex` for crawlers.
	Unlisted,
	/// Excluded from production entirely, whether exported or live-served.
	Draft,
}

impl RouteMeta {
	/// Whether the page is listed: in a sitemap, a feed, a search index, and to
	/// a crawler. False for both [`Unlisted`](RouteVisibility::Unlisted) and
	/// [`Draft`](RouteVisibility::Draft) pages.
	pub fn is_listed(&self) -> bool {
		self.visibility == RouteVisibility::Public
	}

	/// Whether the page is unfinished, ie must not reach production.
	pub fn is_draft(&self) -> bool { self.visibility == RouteVisibility::Draft }

	/// Whether a reader browsing the site finds the page, ie it appears in a
	/// sidebar or a route index: an [`Unlisted`](RouteVisibility::Unlisted) page
	/// is reachable by its link alone, and a [`Draft`](RouteVisibility::Draft)
	/// is previewed everywhere but production, matching static export.
	pub fn is_browsable(&self, is_prod: bool) -> bool {
		match self.visibility {
			RouteVisibility::Public => true,
			RouteVisibility::Unlisted => false,
			RouteVisibility::Draft => !is_prod,
		}
	}

	/// The sidebar label: explicit [`sidebar_label`](Self::sidebar_label), else
	/// the document's [`title`](DocMeta::title).
	pub fn sidebar_label<'a>(
		&'a self,
		doc: Option<&'a DocMeta>,
	) -> Option<&'a str> {
		self.sidebar_label
			.as_deref()
			.or_else(|| doc?.title.as_deref())
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
	pub fn apply_slug(&self, route_path: &RelPath) -> Result<RelPath> {
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
		RelPath::from_segments(&segments).xok()
	}

	/// Declare onto `declarations` the defaults a content file's NAME implies: a
	/// leading `<number>-` on the filename sets [`order`](Self::order), unless the
	/// document declared one. A document declaring only its [`DocMeta`] gets a
	/// [`RouteMeta`] declaration to carry it; one declaring nothing at all gets
	/// no metadata invented for it.
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
		file: &RelPath,
	) {
		let Some(order) = file
			.file_stem()
			.and_then(|stem| stem.split_once('-'))
			.and_then(|(prefix, _)| prefix.parse::<u64>().ok())
		else {
			return;
		};
		let name = type_ext::short_name::<Self>();
		if !declarations.is_empty()
			&& !declarations.0.iter().any(|named| named.name == name)
		{
			declarations.0.push(NamedLiteral {
				name: name.clone(),
				fields: NamedFields::Unit,
			});
		}
		declarations.declare_default(
			&name,
			"order",
			DataLiteral::Scalar(Value::Uint(order)),
		);
	}
}

#[cfg(all(test, feature = "bsx"))]
mod test {
	use super::*;
	use bevy::reflect::TypeRegistry;

	/// Resolve a frontmatter block exactly as a scan does, its `[RouteMeta]`
	/// table naming this component.
	fn parse(content: &str) -> RouteMeta {
		let mut registry = TypeRegistry::default();
		registry.register::<RouteMeta>();
		Frontmatter::parse(content, MediaType::Toml)
			.unwrap()
			.declarations(&type_ext::short_name::<DocMeta>())
			.get::<RouteMeta>(&registry)
			.unwrap()
			.unwrap_or_default()
	}

	#[beet_core::test]
	fn frontmatter_reads_its_table() {
		let meta = parse(
			"title = \"Getting Started\"\n\n[RouteMeta]\nvisibility = \"draft\"\norder = 2\nexpanded = true",
		);
		meta.is_draft().xpect_true();
		meta.is_listed().xpect_false();
		meta.order.unwrap().xpect_eq(2);
		meta.expanded.unwrap().xpect_true();
		let doc = DocMeta {
			title: Some("Getting Started".into()),
			..default()
		};
		// no explicit sidebar_label, so the label falls back to the title
		meta.sidebar_label(Some(&doc))
			.unwrap()
			.xpect_eq("Getting Started");
		RouteMeta::default().sidebar_label(None).xpect_none();
	}

	#[beet_core::test]
	fn apply_slug_renames_last_segment() {
		let meta = RouteMeta {
			slug: Some("full-stack-bevy".into()),
			..default()
		};
		meta.apply_slug(&RelPath::new("blog/1-full-stack-bevy"))
			.unwrap()
			.xpect_eq(RelPath::new("blog/full-stack-bevy"));
		// no slug declared, the filename-derived path stands
		RouteMeta::default()
			.apply_slug(&RelPath::new("blog/post-1"))
			.unwrap()
			.xpect_eq(RelPath::new("blog/post-1"));
		// a nested slug is a loud error, never a silent re-parent
		RouteMeta {
			slug: Some("blog/nested".into()),
			..default()
		}
		.apply_slug(&RelPath::new("blog/post-1"))
		.unwrap_err()
		.to_string()
		.xpect_contains("single path segment");
	}

	#[beet_core::test]
	fn file_defaults_order_by_number() {
		let mut registry = TypeRegistry::default();
		registry.register::<RouteMeta>();
		let order = |file: &str, frontmatter: &str| {
			let mut declarations =
				Frontmatter::parse(frontmatter, MediaType::Toml)
					.unwrap()
					.declarations(&type_ext::short_name::<DocMeta>());
			RouteMeta::declare_file_defaults(
				&mut declarations,
				&RelPath::new(file),
			);
			declarations.get::<RouteMeta>(&registry).unwrap()?.order
		};
		order("blog/10-later.md", "title = \"Later\"")
			.unwrap()
			.xpect_eq(10);
		// an explicit frontmatter order wins over the filename
		order("blog/10-later.md", "[RouteMeta]\norder = 3")
			.unwrap()
			.xpect_eq(3);
		// an unnumbered filename stays unordered
		order("docs/intro.md", "title = \"Intro\"")
			.is_none()
			.xpect_true();
		// a document declaring nothing gets no metadata invented for it
		order("blog/10-later.md", "").is_none().xpect_true();
	}
}
