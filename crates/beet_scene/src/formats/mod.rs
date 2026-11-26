//! The document formats, each a [`DocParser`] in the one registry
//! ([`DocParsers`]): bytes in, a scene body out, touching no world.
//!
//! The formats here build plain scenes: elements, values, and the components a
//! document names, resolved by reflection. A format that builds a UI crate's
//! own types (an office file's styles, a diagram) lives in that crate's
//! `formats` module, `beet_ui`'s, registered into the same registry.
//!
//! - [`bsx`] (`bsx`): BSX, HTML and XML, their directives and the one lowering
//!   every markup format shares.
//! - `markdown` (`markdown`): markdown and mdx, embedding BSX.
//! - `frontmatter`: the leading block a document declares its root's
//!   components in, read by markdown and plain text alike.
//! - `structured_parser`: json, toml and postcard.
//! - `plain_text_parser` and `script_parser`: text and javascript.
//! - `fence_params`: what a markdown fence becomes, and its params.
#[cfg(doc)]
use crate::prelude::*;

#[cfg(feature = "bsx")]
pub mod bsx;
mod doc_parsers;
mod fence_params;
mod frontmatter;
#[cfg(feature = "markdown")]
mod markdown;
mod plain_text_parser;
mod script_parser;
mod structured_parser;

#[cfg(feature = "bsx")]
pub use bsx::*;
pub use doc_parsers::*;
pub use fence_params::*;
pub use frontmatter::*;
#[cfg(feature = "markdown")]
pub use markdown::*;
pub use plain_text_parser::*;
pub use script_parser::*;
pub use structured_parser::*;
