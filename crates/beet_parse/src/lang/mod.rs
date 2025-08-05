//! Module for `script`, `style`, `code` and any other language snippets
//! that require special processing like deduplication or modification.
mod extract_inner_text;
mod extract_lang_nodes;
#[cfg(feature = "css")]
mod parse_lightning;
pub use extract_inner_text::*;
pub use extract_lang_nodes::*;
#[cfg(feature = "syntect")]
pub use parse_syntect::*;
#[cfg(feature = "syntect")]
mod parse_syntect;
#[cfg(feature = "css")]
pub use parse_lightning::*;
