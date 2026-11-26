//! The format registry ([`DocParsers`]), the formats whose document is no
//! markup, and a document of any format parsed into a world.
#[cfg(doc)]
use crate::prelude::*;
mod doc_parsers;
mod fence_params;
mod parse_context;
mod plain_text_parser;
mod script_parser;
mod structured_parser;

pub use doc_parsers::*;
pub use fence_params::*;
pub use parse_context::*;
pub use plain_text_parser::*;
pub use script_parser::*;
pub use structured_parser::*;
