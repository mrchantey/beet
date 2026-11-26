//! Markdown and mdx as scene bodies, and the frontmatter either leads with.
//!
//! `pulldown-cmark` reads the prose (headings, lists, emphasis, code) and
//! `tree_builder` interleaves it with embedded markup, which the BSX fragment
//! primitive ([`BsxFragmentToken::parse_fragment`]) reads, into the
//! [`BsxNode`] tree every markup format lowers from. So a component, a `bx:`
//! directive or a spread in markdown is the one it is in BSX, in one tree.
//!
//! A fence is code, unless it is the document it holds: its first word names a
//! format, by media type or by extension (`text/html`, `html`), and it says
//! `render`, or the format renders a fence by default
//! ([`DocParser::renders_fence`], a diagram) and it does not say
//! `render=false`. What it says beyond that are [`FenceParams`], written as a
//! BSX tag's attributes.
//!
//! A document's frontmatter is its root's components ([`Frontmatter`]), read by
//! the plain text fallback too, so it needs no `markdown` feature.
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use beet_core::prelude::*;

mod frontmatter;
#[cfg(feature = "markdown")]
mod markdown_parser;
#[cfg(feature = "markdown")]
mod tree_builder;

pub use frontmatter::*;
#[cfg(feature = "markdown")]
pub use markdown_parser::*;
