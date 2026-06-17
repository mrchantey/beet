//! The BSX (no-code markup) styling surface: the runtime bridge that lets a
//! markup author declare styles the typed Rust API expresses through types.
//!
//! - [`rule_markup`]: the `<Rule>` tag, a named [`Rule`](crate::token::Rule)
//!   declared at runtime;
//! - [`inline_style`]: the `bx:style` directive, the no-code analogue of the
//!   `inline_class!` macro;
//! - [`prop_name`]: the kebab-property and colour-role name maps bridging markup
//!   strings to the typed [`common_props`](crate::style::common_props) tokens.
mod inline_style;
mod prop_name;
mod rule_markup;
pub use inline_style::*;
pub use prop_name::*;
pub use rule_markup::*;
