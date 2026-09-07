//! Source text to syntax tree: the cursor, the markup parser, the value
//! grammar, and the source-format registry that chooses between them.

mod ast;
mod cursor;
mod fragment;
mod parse;
mod template_format;
mod value;

pub use ast::*;
pub use fragment::*;
pub use parse::*;
pub use template_format::*;
