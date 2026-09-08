//! The one page walk the syndication routes share.

use crate::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;
use beet_ui::prelude::*;

/// A page a syndication route lists: where it serves and what it was authored
/// with.
#[derive(Debug, Clone)]
pub(crate) struct SyndicationPage {
	/// The route path within the router's url space, ie `blog/ecs-router`.
	pub path: SmolPath,
	/// The page's authored metadata. Defaulted for a page that declared none,
	/// which is public like any other.
	pub meta: PageMeta,
}

/// Everything a syndication route serializes from: the pages in its scope, the
/// scope itself, and the package identity their urls and titles resolve
/// against.
///
/// Cloned out of the world by [`SyndicationQuery::scope`] so a handler can
/// serialize (and, for a full-content feed, render pages) without holding the
/// world.
pub(crate) struct SyndicationScope {
	/// The listed pages under this route's scope, in route-tree order.
	pub pages: Vec<SyndicationPage>,
	/// The path this route is declared at, ie the url a feed names as its own
	/// scope root.
	pub root: SmolPath,
	/// The site identity: the fallback title and description, and the origin
	/// every absolute url is joined onto.
	pub package: PackageConfig,
	/// The entity owning this url space's [`RouteTree`], ie where a
	/// full-content document re-enters dispatch to render a page.
	pub router: Entity,
}

impl SyndicationScope {
	/// A page path as an absolute url, ie
	/// [`PackageConfig::absolute_url`] against this site's origin.
	pub fn url(&self, path: &SmolPath) -> Result<String> {
		self.package.absolute_url(path.as_str())
	}
}

/// The route-tree walk every syndication route shares: from the declaring
/// route, find the url space it belongs to, scope to the subtree it was
/// declared in, and collect the listed pages beneath it.
///
/// A [`SystemParam`] rather than an ad-hoc traversal because four async
/// handlers read the same three-query shape through `with_state`.
#[derive(SystemParam)]
pub(crate) struct SyndicationQuery<'w, 's> {
	trees: AncestorQuery<'w, 's, &'static RouteTree>,
	paths: Query<'w, 's, &'static PathPattern>,
	metas: Query<'w, 's, &'static PageMeta>,
	package: Res<'w, PackageConfig>,
}

impl SyndicationQuery<'_, '_> {
	/// The scope of the syndication route at `route`.
	///
	/// # Errors
	/// Errors when the route is in no [`RouteTree`], ie it has no [`Router`]
	/// ancestor.
	///
	/// [`Router`]: crate::prelude::Router
	pub fn scope(&self, route: Entity) -> Result<SyndicationScope> {
		let router = self.trees.get_entity(route)?;
		let tree = self.trees.get(route)?;
		// this route's own position minus its filename segment, so a
		// `<Sitemap/>` serving at `blog/sitemap.xml` covers the `blog` subtree
		// and one at the root covers the whole url space
		let root = self
			.paths
			.get(route)?
			.annotated_path()
			.parent()
			.unwrap_or_default();
		// a draft is invisible in production exactly as it is to dispatch and
		// to static export; the syndication routes additionally drop the
		// unlisted, which serve but are advertised nowhere.
		let is_prod = BootstrapConfig::get().is_prod();
		let pages = tree
			.find_subtree(&root.segments())
			.unwrap_or(tree)
			.flatten_nodes()
			.into_iter()
			.filter_map(|node| {
				let meta = self.metas.get(node.entity).ok();
				(node.is_public_page(meta, is_prod)
					&& meta.is_none_or(PageMeta::is_listed))
				.then(|| SyndicationPage {
					path: node.path.annotated_path(),
					meta: meta.cloned().unwrap_or_default(),
				})
			})
			.collect();
		SyndicationScope {
			pages,
			root,
			package: self.package.clone(),
			router,
		}
		.xok()
	}
}

/// The fixture site every syndication test serializes, so the four documents
/// are asserted against one authored surface rather than four look-alikes.
#[cfg(test)]
pub(crate) mod test_fixtures {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_ui::prelude::*;

	/// A router world whose [`PackageConfig`] names the site the syndication
	/// routes resolve their urls and fall back to their titles from.
	pub fn syndication_world(homepage: Option<&str>) -> World {
		let mut world = (AsyncPlugin, RouterPlugin).into_world();
		world.insert_resource(PackageConfig {
			title: "Beet".into(),
			description: "A malleable engine for sovereign software".into(),
			homepage: homepage.map(SmolStr::new),
			..default()
		});
		world
	}

	/// A page route at `path` carrying `meta`. The body is irrelevant:
	/// syndication reads the route tree and the metadata, never the render.
	fn page(path: &str, meta: PageMeta) -> impl Bundle {
		(
			render_action::fixed_func_route(path, || rsx! { <p>"page"</p> }),
			PageRoute,
			meta,
		)
	}

	/// A published post's frontmatter.
	fn post(title: &str, created: &str) -> PageMeta {
		PageMeta {
			title: Some(title.into()),
			description: Some(format!("all about {title}")),
			author: Some("Pete Hayman".into()),
			created: Timestamp::parse_date(created),
			..default()
		}
	}

	/// A router serving a home page and a `blog` subtree of three posts — one
	/// public and edited since publication, one public, one unlisted — plus a
	/// draft, and whatever syndication markup the case declares.
	pub fn spawn_syndication_router(
		world: &mut World,
		markup: Snippet,
	) -> Entity {
		let root = world
			.spawn((Router, children![
				page("", PageMeta {
					title: Some("Beet".into()),
					..default()
				}),
				page("blog", PageMeta {
					title: Some("Blog".into()),
					..default()
				}),
				page("blog/full-stack-bevy", {
					let mut meta = post("Full Stack Bevy", "2025-07-11");
					meta.updated = Timestamp::parse_date("2025-09-01");
					meta
				}),
				page("blog/ecs-router", post("ECS Router", "2025-08-09")),
				page("blog/hidden", {
					let mut meta = post("Hidden", "2025-08-20");
					meta.visibility = PageVisibility::Unlisted;
					meta
				}),
				page("blog/wip", {
					let mut meta = post("Work In Progress", "2025-08-25");
					meta.visibility = PageVisibility::Draft;
					meta
				}),
			]))
			.flush();
		world
			.spawn_template(Snippet::from_bundle((ChildOf(root), markup)))
			.unwrap();
		world.flush();
		root
	}
}
