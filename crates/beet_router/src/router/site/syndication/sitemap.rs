//! The crawler index route: `sitemap.xml`.

use super::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The site's `sitemap.xml`: a Static `GET` route listing every public page
/// beneath it, each with the date it last changed.
///
/// Declared as `<Sitemap/>`. Scoped by position like every syndication route,
/// so one under the router root covers the whole url space and one inside
/// `<Route path="docs">` indexes only the docs.
///
/// Only [`Public`](beet_ui::prelude::PageVisibility::Public) pages appear:
/// unlisted pages are reachable by their link alone, and drafts are absent from
/// production entirely.
///
/// # Errors
/// Dispatch fails when [`PackageConfig::homepage`] is unset. A `<loc>` is
/// resolved against nothing, so a relative one is not a degraded sitemap but an
/// invalid one.
#[template]
pub fn Sitemap() -> impl Bundle {
	(
		route::exchange(
			"sitemap.xml",
			Action::<Request, Response>::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let scope = cx
						.caller
						.with_state::<SyndicationQuery, _>(|entity, query| {
							query.scope(entity)
						})
						.await??;
					Response::ok_body(
						SitemapEntry::document(&scope)?,
						MediaType::Xml,
					)
					.xok()
				},
			),
		),
		HttpMethod::Get,
		ExportStrategy::Static,
	)
}

/// One `<url>` in the sitemap: where a page lives and when it last changed.
///
/// Built before any string is written so the collection is testable on its own,
/// and so a page whose url cannot be resolved fails the whole document rather
/// than silently dropping out of it.
///
/// `changefreq` and `priority` are deliberately absent: Google has ignored both
/// for years, so they would be noise a site owner has to keep truthful.
#[derive(Debug, Clone, PartialEq)]
struct SitemapEntry {
	/// The page's absolute url.
	loc: Url,
	/// The page's last substantive edit as a W3C date, ie `YYYY-MM-DD`. Absent
	/// on a page whose frontmatter names neither `updated` nor `created`.
	lastmod: Option<String>,
}

impl SitemapEntry {
	/// The entries for every page in `scope`, in route-tree order.
	fn collect(scope: &SyndicationScope) -> Result<Vec<Self>> {
		scope
			.pages
			.iter()
			.map(|page| {
				Self {
					loc: scope.url(&page.path)?,
					lastmod: page
						.meta
						.last_modified()
						.map(|date| date.format_date()),
				}
				.xok()
			})
			.collect()
	}

	/// The whole `sitemap.xml` document for `scope`.
	fn document(scope: &SyndicationScope) -> Result<String> {
		let mut out = String::from(PROLOG);
		out.push_str(
			"<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
		);
		for entry in Self::collect(scope)? {
			out.push_str("\t<url>\n");
			out.push_str(&element(2, "loc", &entry.loc.to_string()));
			out.push_str(&element_opt(2, "lastmod", entry.lastmod));
			out.push_str("\t</url>\n");
		}
		out.push_str("</urlset>\n");
		out.xok()
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn lists_public_pages() {
		let mut world = syndication_world(Some("https://beet.org"));
		let root = spawn_syndication_router(&mut world, rsx! { <Sitemap/> });
		let response = world
			.entity_mut(root)
			.exchange(Request::get("sitemap.xml"))
			.await;
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Xml);
		// the unlisted and draft pages are absent, and the `sitemap.xml` route
		// itself never lists itself (it is no page)
		response.unwrap_str().await.xpect_snapshot();
	}

	/// A `<loc>` must be absolute, so a site that names no origin fails loudly
	/// at dispatch rather than serving urls no crawler can resolve.
	#[beet_core::test]
	async fn requires_a_homepage() {
		let mut world = syndication_world(None);
		let root = spawn_syndication_router(&mut world, rsx! { <Sitemap/> });
		world
			.entity_mut(root)
			.exchange(Request::get("sitemap.xml"))
			.await
			.into_result()
			.await
			.unwrap_err()
			.to_string()
			.xpect_contains("PackageConfig.homepage");
	}
}
