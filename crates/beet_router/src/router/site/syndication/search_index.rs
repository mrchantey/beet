//! The client-search route: `search-index.json`.

use super::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The site's search index: a Static `GET` route serving every public page
/// beneath it as one JSON array, for a client-side search that fetches the
/// document once and queries it in the browser.
///
/// Declared as `<SearchIndex/>`, and scoped by position like every syndication
/// route, so one inside `<Route path="docs">` indexes the docs alone.
///
/// # Errors
/// Dispatch fails when [`PackageConfig::homepage`] is unset, like every
/// absolute-url document (see [`Sitemap`]).
#[template]
pub fn SearchIndex() -> impl Bundle {
	(
		route::exchange(
			"search-index.json",
			Action::<Request, Response>::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let scope = cx
						.caller
						.with_state::<SyndicationQuery, _>(|entity, query| {
							query.scope(entity)
						})
						.await??;
					Response::ok_body(
						SearchEntry::document(&cx.caller.world(), &scope)
							.await?,
						MediaType::Json,
					)
					.xok()
				},
			),
		),
		HttpMethod::Get,
		ExportStrategy::Static,
	)
}

/// One indexed page.
///
/// Hand-serialized through [`json_ext`] rather than derived: a `Serialize` impl
/// would put this whole route behind the `json` feature, and a site's search
/// index should exist in every build that serves the site.
#[derive(Debug, Clone, PartialEq)]
struct SearchEntry {
	/// The page's absolute url, which is both its identity and where a result
	/// links to.
	url: String,
	title: Option<String>,
	description: Option<String>,
	author: Option<SmolStr>,
	/// The publication date as `YYYY-MM-DD`, ie sortable as text.
	created: Option<String>,
	/// The page's rendered prose, which is what makes this a FULL-text index
	/// rather than a list of titles. Absent when the page failed to render (see
	/// [`PageContent::render`]).
	body: Option<String>,
}

impl SearchEntry {
	/// The entries for every page in `scope`, in route-tree order, each
	/// rendered in-process for its body text.
	///
	/// Unlike the feed this renders EVERY listed page, which a static export
	/// pays once at export time and a live site pays per fetch of a document
	/// meant to be cached; the timing is logged rather than pre-optimized.
	async fn document(
		world: &AsyncWorld,
		scope: &SyndicationScope,
	) -> Result<String> {
		let started = Instant::now();
		let mut entries = Vec::with_capacity(scope.pages.len());
		for page in &scope.pages {
			let content =
				PageContent::render(world, scope.router, &page.path).await;
			entries.push(Self {
				url: scope.url(&page.path)?,
				title: page.meta.title.clone(),
				description: page.meta.description.clone(),
				author: page.meta.author.clone(),
				created: page.meta.created.map(|created| created.format_date()),
				body: content.map(|content| content.text()),
			});
		}
		debug!(
			"syndication: indexed {} pages in {:?}",
			entries.len(),
			started.elapsed()
		);
		json_ext::array(entries.iter().map(Self::to_json)).xok()
	}

	/// This entry as a json object, its unauthored fields absent rather than
	/// `null`.
	fn to_json(&self) -> String {
		json_ext::object([
			json_ext::member_opt("url", Some(&self.url)),
			json_ext::member_opt("title", self.title.as_ref()),
			json_ext::member_opt("description", self.description.as_ref()),
			json_ext::member_opt("author", self.author.as_ref()),
			json_ext::member_opt("created", self.created.as_ref()),
			json_ext::member_opt("body", self.body.as_ref()),
		])
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	#[beet_core::test]
	async fn indexes_public_pages() {
		let mut world = syndication_world(Some("https://beet.org"));
		let root =
			spawn_syndication_router(&mut world, rsx! { <SearchIndex/> });
		let response = world
			.entity_mut(root)
			.exchange(Request::get("search-index.json"))
			.await;
		response
			.parts
			.headers
			.get::<header::ContentType>()
			.unwrap()
			.unwrap()
			.xpect_eq(MediaType::Json);
		response.unwrap_str().await.xpect_snapshot();
	}
}
