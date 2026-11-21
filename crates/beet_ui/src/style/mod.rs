//! Target-agnostic styling: classes, declarations, design tokens and the
//! cascade that resolves them for every target.
//!
//! # The cascade
//!
//! Every [`Rule`](crate::prelude::Rule) ranks by its
//! [`RuleOrigin`](crate::prelude::RuleOrigin) (a user agent default, an author
//! rule, a direct look), then its selector's specificity, then its order, as
//! CSS's cascade does. The output is target agnostic: [`ComputedStyle`]'s
//! components ([`VisualStyle`], [`LayoutStyle`], [`BoxStyle`],
//! [`PositionStyle`], ..) say nothing about where they are drawn, and each
//! target maps them onto its own, the terminal onto cells, a document writer
//! onto a file's formatting, a window onto its nodes.
//!
//! A target declares what it is with a [`Medium`](crate::prelude::Medium)
//! beside its [`MediaViewport`](crate::prelude::MediaViewport) on the surface
//! entity, and every `@media` rule matches against the nearest surface an
//! element renders under ([`MediaQuery::matches`](crate::prelude::MediaQuery::matches)):
//! the charcell buffers are [`Terminal`](crate::prelude::Medium::Terminal),
//! as is a tree under no surface. A live surface's tree is resolved into its
//! components by the `resolve_styles` system; a writer resolves a tree for a
//! file through the pure [`CascadeQuery`], stating its medium, viewport and
//! colour scheme in a [`MediaContext`](crate::prelude::MediaContext) and
//! writing nothing, so the same tree may be on screen as it writes. The web
//! needs neither: the browser cascades the serialized stylesheet itself.
//!
//! Conventions:
//! - Colocate a widget's own classes with the widget: a class only that widget
//!   emits is declared and styled beside it, never in a central rules file.
//!   The exception is the *shared* vocabulary in [`material::classes`], the
//!   names more than one widget emits and any styling system may implement; a
//!   widget reaching `classes::CARD_FILLED` is emitting a shared name, not
//!   surrendering ownership of its own styling.
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
pub(crate) use color_scheme::*;
pub use css::*;
pub(crate) use elements::*;
pub use resolve_styles::*;
pub use style_plugin::*;
pub use style_query::*;
pub use values::*;
