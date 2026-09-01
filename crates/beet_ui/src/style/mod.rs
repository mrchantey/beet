//! Target-agnostic styling: classes, declarations, design tokens and the
//! cascade that resolves them for both the web and charcell targets.
//!
//! Conventions:
//! - Colocate a widget's classes with the widget, never in a central rules
//!   file. A widget owns its styling.
//! - A widget with only one class uses `inline_class!` rather than registering
//!   a named rule. A plain declaration is a `(prop, value)` pair; to point a
//!   prop at a design token use `Declaration::token(prop, value)`, ie
//!   `Declaration::token(BackgroundColor, colors::InverseSurface)`.
//! - Put the `inline_class!` in a helper function (eg `fn toast_style() ->
//!   impl Bundle`) when it is more than two tokens long; keep it inline at the
//!   call site otherwise.
mod animate;
mod bsx_style;
mod color_scheme;
pub mod common_props;
mod css;
mod elements;
pub mod material;
mod resolve_styles;
mod style_plugin;
mod style_query;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
pub mod syntax;
mod values;
pub use animate::*;
pub use bsx_style::*;
pub use color_scheme::*;
pub use css::*;
pub(crate) use elements::*;
pub(crate) use resolve_styles::*;
pub use style_plugin::*;
pub use style_query::*;
pub use values::*;
