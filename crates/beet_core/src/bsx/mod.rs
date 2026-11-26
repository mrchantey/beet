//! BSX markup as a syntax tree: the cursor, the markup parser and the value
//! grammar.
//!
//! BSX is a superset of HTML and XML. [`BsxParseConfig`] selects the dialect a
//! source is read in, the HTML and XML dialects disabling the BSX surface, so
//! the one parser reads a page, a scene document, an Office part and a
//! protocol's xml answer alike. What a tree becomes in a scene is
//! `beet_scene`'s lowering.

mod ast;
mod cursor;
mod fragment;
mod parse;
mod span;
mod value;

pub use ast::*;
pub use fragment::*;
pub use parse::*;
pub use span::*;
