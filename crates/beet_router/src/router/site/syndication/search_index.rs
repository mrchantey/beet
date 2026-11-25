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
	url: Url,
	title: Option<String>,
	description: Option<String>,
	authors: Vec<SmolStr>,
	/// The publication date as `YYYY-MM-DD`, ie sortable as text.
	created: Option<String>,
	/// The page's rendered prose, which is what makes this a FULL-text index
	/// rather than a list of titles. Absent when the page failed to render (see
	/// [`SyndicationScope::render_content`]).
	body: Option<String>,
}

impl SearchEntry {
	/// How much plain text one page contributes to a search index. Client-side
	/// search matches on the opening prose in practice, and the index is
	/// fetched whole by every visitor, so the tail is cost without benefit.
	const TEXT_LIMIT: usize = 8 * 1024;

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
			let body = scope
				.render_content(world, page, MediaType::Text)
				.await
				.map(|text| Self::reduce(&text));
			entries.push(Self {
				url: scope.url(&page.path)?,
				title: page.meta.title.clone(),
				description: page.meta.description.clone(),
				authors: page.meta.authors.clone(),
				created: page.meta.created.map(|created| created.to_string()),
				body,
			});
		}
		debug!(
			"syndication: indexed {} pages in {:?}",
			entries.len(),
			started.elapsed()
		);
		Value::new_list(entries.iter().map(Self::to_value))
			.to_json_string()
			.xok()
	}

	/// A page's plain text as an index carries it: whitespace collapsed to
	/// single spaces and capped at [`TEXT_LIMIT`](Self::TEXT_LIMIT), cut at a
	/// char boundary so a multi-byte glyph is never split down the middle.
	fn reduce(text: &str) -> String {
		let mut text = text.split_whitespace().collect::<Vec<_>>().join(" ");
		if text.len() > Self::TEXT_LIMIT {
			let end = (0..=Self::TEXT_LIMIT)
				.rev()
				.find(|index| text.is_char_boundary(*index))
				.unwrap_or_default();
			text.truncate(end);
		}
		text
	}

	/// This entry as a json object. An unauthored field is left OUT rather than
	/// written `null`: a client branches on presence either way, and a document
	/// every visitor fetches stays small.
	fn to_value(&self) -> Value {
		let mut map = Map::default();
		map.insert("url", Value::str(self.url.to_string()));
		for (key, value) in [
			("title", self.title.clone()),
			("description", self.description.clone()),
			("created", self.created.clone()),
			("body", self.body.clone()),
		] {
			if let Some(value) = value {
				map.insert(key, Value::str(value));
			}
		}
		if !self.authors.is_empty() {
			map.insert(
				"authors",
				Value::new_list(
					self.authors
						.iter()
						.map(|author| Value::str(author.as_str())),
				),
			);
		}
		Value::Map(map)
	}
}

#[cfg(test)]
mod test {
	use super::*;

	/// The reduction keeps the prose, whitespace collapsed, and caps it at a
	/// char boundary.
	#[beet_core::test]
	fn reduces_the_text() {
		SearchEntry::reduce("Hello &\n\n  welcome\tto\nthe garden\n")
			.xpect_eq("Hello & welcome to the garden".to_string());
		let reduced = SearchEntry::reduce(&"é".repeat(SearchEntry::TEXT_LIMIT));
		reduced.len().xpect_eq(SearchEntry::TEXT_LIMIT);
		reduced.chars().all(|char| char == 'é').xpect_true();
	}

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
