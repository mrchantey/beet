//! Authored literals: [`DataLiteral`], the value model every authored source
//! resolves against a type, and [`LiteralParser`], the one table of how
//! human-authored input parses into a reflected value, and the entries beet
//! seeds into it.

mod builtins;
mod data_literal;
pub use data_literal::*;
mod literal_parser;
pub use literal_parser::*;
