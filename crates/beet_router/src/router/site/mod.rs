//! The page chrome a route renders through: the layout middleware, the site
//! shell, the nav rail and index, the help/404 pages, and the article header.
//!
//! std-only throughout: every module here builds on the `beet_ui` scene
//! pipeline.

#[cfg(feature = "std")]
mod app_shell;
// the article chrome (`<ArticleHeader/>`, `<YouTubeEmbed/>`) a layout places
// above a post's body. std-only: it renders through the beet_ui widget layer.
#[cfg(feature = "std")]
mod article_header;
#[cfg(feature = "std")]
mod help;
// `<IconSet/>`: the icons a browser and a PWA ask for, derived from one image
#[cfg(feature = "std")]
mod icon_set;
#[cfg(feature = "std")]
mod layout;
#[cfg(feature = "std")]
mod route_index;
#[cfg(feature = "std")]
mod sidebar;
// `<ScriptDerivation/>`: files a script derives from resources of the world
#[cfg(all(feature = "std", feature = "scripting"))]
mod script_derivation;
// `<ServerIsland>`: a region of a page rendered live, and `<ServerIslands/>`,
// the route answering each one
#[cfg(feature = "std")]
mod server_island;
#[cfg(feature = "std")]
mod site_layout;
// `<SocialCards/>`: a link preview drawn for every page, on the icon set's
// image toolchain
#[cfg(feature = "std")]
mod social_cards;
// the machine-readable faces of a site (`robots.txt`, `sitemap.xml`, `rss.xml`,
// `search-index.json`): ordinary static routes over the `RouteTree`.
#[cfg(feature = "std")]
mod syndication;
// what a layout, or a thread route, names: a document or a rust template
#[cfg(feature = "std")]
mod template_source;
// the browser-wasm page template `<Wasm>`: serve-side, building a page that
// boots a wasm `beet` binary with the launch it names. A plain synchronous
// template, so it renders inside a route's content (std-gated like the rest of
// the render pipeline).
#[cfg(feature = "std")]
mod wasm;

#[cfg(feature = "std")]
pub use app_shell::*;
#[cfg(feature = "std")]
pub use article_header::*;
#[cfg(feature = "std")]
pub use help::*;
#[cfg(feature = "std")]
pub use icon_set::*;
#[cfg(all(feature = "std", feature = "scripting"))]
pub use script_derivation::*;
#[cfg(feature = "std")]
pub use layout::*;
#[cfg(feature = "std")]
pub use route_index::*;
#[cfg(feature = "std")]
pub use server_island::*;
#[cfg(feature = "std")]
pub use sidebar::*;
#[cfg(feature = "std")]
pub use site_layout::*;
#[cfg(feature = "std")]
pub use social_cards::*;
#[cfg(feature = "std")]
pub use syndication::*;
#[cfg(feature = "std")]
pub use template_source::*;
#[cfg(feature = "std")]
pub use wasm::*;
