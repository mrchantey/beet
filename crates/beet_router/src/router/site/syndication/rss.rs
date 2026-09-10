//! The subscription route: `rss.xml`.

use super::*;
use crate::prelude::*;
use beet_action::prelude::*;
use beet_core::prelude::*;
use beet_net::prelude::*;

/// The site's RSS 2.0 feed: a Static `GET` route serving the newest public
/// pages beneath it, newest first.
///
/// Declared as `<RssFeed/>`, and scoped by position like every syndication
/// route, so `<RssFeed title="The Full Moon Harvest"/>` inside
/// `<Route path="blog">` serves at `/blog/rss.xml` and feeds only the posts.
///
/// Only dated pages appear: a feed entry with no publication date is one no
/// reader can place, and it is `created` that makes a page an *entry* rather
/// than a standing page, so an undated index drops out on its own.
///
/// # Errors
/// Dispatch fails when [`PackageConfig::homepage`] is unset, like every
/// absolute-url document (see [`Sitemap`]).
#[template]
pub fn RssFeed(
	/// The channel title. Defaults to the [`PackageConfig`] title.
	#[prop]
	title: Option<String>,
	/// The channel description. Defaults to the [`PackageConfig`] description.
	#[prop]
	description: Option<String>,
	/// How many entries the feed carries, newest first.
	///
	/// A feed is "what's new" where the sitemap is "everything", so capping it
	/// loses nothing and keeps a long-lived blog's feed small.
	#[prop(default = 20_usize)]
	limit: usize,
	/// Carry each entry's whole rendered body as `<content:encoded>`, so a
	/// reader shows the post rather than a teaser. On by default: a feed a
	/// person can actually read in their reader is the point of publishing one.
	///
	/// Turn it off for a summary-only feed, which also skips rendering a page
	/// per entry on every fetch.
	#[prop(default = true)]
	full_content: bool,
) -> impl Bundle {
	(
		route::exchange(
			"rss.xml",
			Action::<Request, Response>::new_async(
				async move |cx: ActionContext<Request>| -> Result<Response> {
					let scope = cx
						.caller
						.with_state::<SyndicationQuery, _>(|entity, query| {
							query.scope(entity)
						})
						.await??;
					let channel = FeedChannel {
						title: title.clone(),
						description: description.clone(),
						limit,
						full_content,
					};
					Response::ok_body(
						channel.document(&cx.caller.world(), &scope).await?,
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

/// The feed's own identity: what the `<channel>` says about itself, and how
/// many `<item>`s it carries.
struct FeedChannel {
	title: Option<String>,
	description: Option<String>,
	limit: usize,
	full_content: bool,
}

impl FeedChannel {
	/// The whole `rss.xml` document for `scope`, rendering each entry's page to
	/// carry its body.
	async fn document(
		&self,
		world: &AsyncWorld,
		scope: &SyndicationScope,
	) -> Result<String> {
		let package = &scope.package;
		let mut out = String::from(PROLOG);
		// the content module's namespace, declared only by a feed that carries
		// bodies: an unused namespace on a summary feed is noise
		out.push_str(match self.full_content {
			true => "<rss version=\"2.0\" xmlns:content=\"http://purl.org/rss/1.0/modules/content/\">\n\t<channel>\n",
			false => "<rss version=\"2.0\">\n\t<channel>\n",
		});
		out.push_str(&element(
			2,
			"title",
			self.title.as_deref().unwrap_or(&package.title),
		));
		out.push_str(&element(2, "link", &scope.url(&scope.root)?.to_string()));
		out.push_str(&element(
			2,
			"description",
			self.description.as_deref().unwrap_or(&package.description),
		));
		for page in FeedItem::entries(scope, self.limit) {
			let content = match self.full_content {
				true => {
					PageContent::render(world, scope.router, &page.path).await
				}
				false => None,
			};
			out.push_str(&FeedItem::new(scope, page, content)?.render());
		}
		out.push_str("\t</channel>\n</rss>\n");
		out.xok()
	}
}

/// One `<item>` in the feed: a dated page, resolved to the absolute url that is
/// both its link and its permanent identity.
#[derive(Debug, Clone, PartialEq)]
struct FeedItem {
	title: String,
	/// The page's absolute url, serving as both `<link>` and `<guid>`.
	link: Url,
	/// The publication date as RFC 2822, the format the RSS spec names.
	pub_date: String,
	description: Option<String>,
	author: Option<SmolStr>,
	/// The page's rendered article markup, absent when the page failed to
	/// render (see [`PageContent::render`]).
	content: Option<String>,
}

impl FeedItem {
	/// The newest `limit` dated pages in `scope`, newest first.
	fn entries(
		scope: &SyndicationScope,
		limit: usize,
	) -> Vec<&SyndicationPage> {
		let mut dated: Vec<(Timestamp, &SyndicationPage)> = scope
			.pages
			.iter()
			.filter_map(|page| Some((page.meta.created?, page)))
			.collect();
		// newest first, ties broken by path so the document is deterministic
		dated.sort_by(|(created_a, page_a), (created_b, page_b)| {
			created_b
				.cmp(created_a)
				.then_with(|| page_a.path.cmp(&page_b.path))
		});
		dated
			.into_iter()
			.take(limit)
			.map(|(_, page)| page)
			.collect()
	}

	/// One entry: `page`'s metadata resolved against `scope`, carrying whatever
	/// `content` its render produced.
	fn new(
		scope: &SyndicationScope,
		page: &SyndicationPage,
		content: Option<PageContent>,
	) -> Result<Self> {
		Self {
			title: page
				.meta
				.title
				.clone()
				.unwrap_or_else(|| page.path.to_string()),
			link: scope.url(&page.path)?,
			// `entries` selected on `created`, so this is never the fallback
			pub_date: page.meta.created.unwrap_or_default().format_rfc2822(),
			description: page.meta.description.clone(),
			author: page.meta.author.clone(),
			content: content.map(|content| content.html),
		}
		.xok()
	}

	/// This item as an `<item>` element.
	fn render(&self) -> String {
		let mut out = String::from("\t\t<item>\n");
		out.push_str(&element(3, "title", &self.title));
		let link = self.link.to_string();
		out.push_str(&element(3, "link", &link));
		// the url is the permanent identity too, which is what a reader
		// deduplicates on across refetches
		out.push_str(&format!(
			"\t\t\t<guid isPermaLink=\"true\">{}</guid>\n",
			escape(&link)
		));
		out.push_str(&element(3, "pubDate", &self.pub_date));
		out.push_str(&element_opt(3, "description", self.description.as_ref()));
		out.push_str(&element_opt(3, "author", self.author.as_ref()));
		// the full body as markup: CDATA rather than escaping, so a reader that
		// shows the source shows html rather than a wall of entities
		if let Some(content) = &self.content {
			out.push_str(&format!(
				"\t\t\t<content:encoded><![CDATA[{}]]></content:encoded>\n",
				// the one sequence CDATA cannot carry, split across two sections
				content.replace("]]>", "]]]]><![CDATA[>")
			));
		}
		out.push_str("\t\t</item>\n");
		out
	}
}

#[cfg(test)]
mod test {
	use crate::prelude::*;
	use beet_core::prelude::*;
	use beet_net::prelude::*;

	/// The feed under the router root: every dated public page, newest first,
	/// with the undated home and blog index dropping out on their own.
	#[beet_core::test]
	async fn feeds_dated_pages() {
		let mut world = syndication_world(Some("https://beet.org"));
		let root = spawn_syndication_router(&mut world, rsx! { <RssFeed/> });
		world
			.entity_mut(root)
			.exchange(Request::get("rss.xml"))
			.await
			.unwrap_str()
			.await
			.xpect_snapshot();
	}

	/// Declared inside `<Route path="blog">` the feed serves at `/blog/rss.xml`
	/// and covers that subtree alone, `limit` keeps it to what is new, and
	/// `full_content=false` makes it a summary feed: no rendered bodies, and no
	/// content namespace declared for bodies that are not there.
	#[beet_core::test]
	async fn scopes_and_limits() {
		let mut world = syndication_world(Some("https://beet.org"));
		let root = spawn_syndication_router(&mut world, rsx! {
			<Route path="blog">
				<RssFeed
					title="The Full Moon Harvest"
					limit=1_usize
					full_content=false/>
			</Route>
		});
		world
			.entity_mut(root)
			.exchange(Request::get("blog/rss.xml"))
			.await
			.unwrap_str()
			.await
			.xpect_snapshot();
	}
}
