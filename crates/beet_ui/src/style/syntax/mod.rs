//! Design tokens and theme defaults for syntax highlighting.
//!
//! Each token corresponds to a tree-sitter capture name like `keyword`,
//! `function`, `string.escape`. Capture names contain dots which become
//! dot-separated segments at lookup time; the most specific match wins
//! (eg `string.escape` matches before `string`).
//!
//! ## Layout
//! - `tokens` — `css_variable!` declarations and the `CssTokenMap`
//! - `theme` — default light/dark colour schemes

pub mod theme;
pub mod tokens;

pub use theme::*;
pub use tokens::*;
