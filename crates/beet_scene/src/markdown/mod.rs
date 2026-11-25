//! Markdown and mdx as scene bodies, and the frontmatter either leads with.
//!
//! `pulldown-cmark` reads the prose (headings, lists, emphasis, code) and
//! `tree_builder` interleaves it with embedded markup, which the BSX fragment
//! primitive ([`BsxFragmentToken::parse_fragment`]) reads, into the
//! [`BsxNode`] tree every markup format lowers from. So a component, a `bx:`
//! directive or a spread in markdown is the one it is in BSX, in one tree.
//!
//! A fence whose info string names a media type with a parser is that format's
//! document, lowered in place ([`FenceInfo`]). The name is a media type
//! (`text/html`, `mermaid`) and never a language (`html`), so a code sample is
//! code.
//!
//! A document's frontmatter is its root's components ([`Frontmatter`]), read by
//! the plain text fallback too, so it needs no `markdown_parser` feature.
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use beet_core::prelude::*;

mod fence_info;
mod frontmatter;
#[cfg(feature = "markdown_parser")]
mod markdown_parser;
#[cfg(feature = "markdown_parser")]
mod tree_builder;

pub use fence_info::*;
pub use frontmatter::*;
#[cfg(feature = "markdown_parser")]
pub use markdown_parser::*;
