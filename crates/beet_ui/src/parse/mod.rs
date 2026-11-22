//! The markup formats this crate parses, and what extends a parsed tree.
//!
//! The format registry is `beet_core`'s [`DocParsers`], where every
//! [`DocParser`] returns a scene body; [`ParsePlugin`] registers BSX, HTML,
//! XML and markdown beside the base formats, and runs the [`PostParseTree`]
//! steps (tables, styles, highlighting) every parse ends in.
#[cfg(feature = "bsx")]
mod bsx;
#[cfg(feature = "bsx")]
pub use bsx::*;
// the document-metadata block a markdown source leads with. Not a markdown-parser
// internal: route discovery reads it to scan a page's declarations without a
// content parse, so it rides `bsx` (for `RootDeclarations`), not `markdown_parser`.
#[cfg(feature = "bsx")]
mod frontmatter;
#[cfg(feature = "bsx")]
pub use frontmatter::*;
#[cfg(feature = "markdown_parser")]
mod markdown;
#[cfg(feature = "markdown_parser")]
pub use markdown::*;
#[cfg(feature = "mermaid")]
mod mermaid;
#[cfg(feature = "mermaid")]
pub use mermaid::*;
mod parse_plugin;
mod span_tracker;
pub use parse_plugin::*;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
mod syntax_highlighting;
pub use span_tracker::*;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
pub use syntax_highlighting::*;
