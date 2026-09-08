//! The machine-readable faces of a site: `robots.txt`, `sitemap.xml`,
//! `rss.xml` and `search-index.json`.
//!
//! Each is an ordinary [`ExportStrategy::Static`] `GET` route spawned by a
//! markup tag, so it serves live and static-exports through the same path any
//! other route takes, and each is opt-in: a beet binary ships none of them
//! until its entry asks for them.
//!
//! Position is the whole configuration surface. A [`Router`] is a url space and
//! routes root at their ancestors, so `<Sitemap/>` under the router root covers
//! the site while `<RssFeed/>` inside `<Route path="blog">` serves at
//! `/blog/rss.xml` and feeds only the posts beneath it. There is no filter
//! config to keep in sync with the tree.
//!
//! [`ExportStrategy::Static`]: beet_core::prelude::ExportStrategy
//! [`Router`]: crate::prelude::Router

mod page_content;
mod robots;
mod rss;
mod search_index;
mod sitemap;
mod syndication_query;
mod xml;

#[allow(unused_imports)]
pub(crate) use page_content::*;
pub use robots::*;
pub use rss::*;
pub use search_index::*;
pub use sitemap::*;
#[cfg(test)]
pub(crate) use syndication_query::test_fixtures::*;
#[allow(unused_imports)]
pub(crate) use syndication_query::*;
#[allow(unused_imports)]
pub(crate) use xml::*;
