//! Module for `script`, `style`, `code` and any other language snippets
//! that require special processing like deduplication or modification.
#[cfg(feature = "css")]
mod parse_lightning;
mod extract_lang_nodes;
pub use extract_lang_nodes::*;
#[cfg(feature = "css")]
pub use parse_lightning::*;
