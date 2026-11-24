//! Source text to syntax tree: the cursor, the markup parser, the value
//! grammar, and [`XmlElement`], an xml document read as a plain element tree.

mod ast;
mod cursor;
mod fragment;
mod parse;
mod value;
mod xml_element;

pub use ast::*;
pub use fragment::*;
pub use parse::*;
pub use xml_element::*;
