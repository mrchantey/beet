//! The page chrome a route renders through: the layout middleware, the site
//! shell, the nav rail and index, the help/404 pages, and the article header.
//!
//! std-only throughout: every module here builds on the `beet_ui` scene
//! pipeline.

// the article chrome (`<ArticleHeader/>`, `<YouTubeEmbed/>`) a layout places
// above a post's body. std-only: it renders through the beet_ui widget layer.
#[cfg(feature = "std")]
mod article_header;
#[cfg(feature = "std")]
mod help;
#[cfg(feature = "std")]
mod layout;
#[cfg(feature = "std")]
mod route_index;
#[cfg(feature = "std")]
mod sidebar;
#[cfg(feature = "std")]
mod site_layout;
// the `<Template src>` include: needs the BSX tag seam + the unified loader. It
// reads through the store as an async pending dependency, so it relies on the
// async runtime that `bsx` (→ `std`) pulls in (the same one `RoutesDir` uses).
#[cfg(all(feature = "bsx", feature = "template_serde"))]
mod template_include;
// the browser-wasm page templates `<Wasm>` + `<MainBsx>`: serve-side, building a
// page that boots a wasm `beet` binary and references its `.bsx` program. Plain
// synchronous templates, so they render inside a route's content (std-gated like
// the rest of the render pipeline).
#[cfg(feature = "std")]
mod wasm;

#[cfg(feature = "std")]
pub use article_header::*;
#[cfg(feature = "std")]
pub use help::*;
#[cfg(feature = "std")]
pub use layout::*;
#[cfg(feature = "std")]
pub use route_index::*;
#[cfg(feature = "std")]
pub use sidebar::*;
#[cfg(feature = "std")]
pub use site_layout::*;
#[cfg(all(feature = "bsx", feature = "template_serde"))]
pub(crate) use template_include::*;
#[cfg(feature = "std")]
pub use wasm::*;
