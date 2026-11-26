//! Markdown and mdx as scene bodies.
//!
//! `pulldown-cmark` reads the prose (headings, lists, emphasis, code) and
//! `tree_builder` interleaves it with embedded markup, which the BSX fragment
//! primitive ([`BsxFragmentToken::parse_fragment`]) reads, into the
//! [`BsxNode`] tree every markup format lowers from. So a component, a `bx:`
//! directive or a spread in markdown is the one it is in BSX, in one tree.
//!
//! A fence is code unless it is the document it holds: its first word names a
//! format, by media type or by extension (`text/html`, `html`), and its
//! `render` is `scene` (or bare), or it does not say and the format renders a
//! fence by default ([`DocParser::fence_render`], a diagram). What it says
//! beyond that are [`FenceParams`], written as a BSX tag's attributes.
//!
//! A document's frontmatter is its root's components ([`Frontmatter`]), shared
//! with plain text in the parent module.
#[cfg(doc)]
use crate::prelude::*;
#[cfg(doc)]
use beet_core::prelude::*;

mod markdown_parser;
mod tree_builder;

pub use markdown_parser::*;
