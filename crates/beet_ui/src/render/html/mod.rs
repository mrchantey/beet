//! HTML rendering: the [`HtmlRenderer`] and its utilities, plus the
//! template-serde wiring.
//!
//! The shared walk/serialize substrate (`node_walker`, `node_renderer`) lives
//! one level up in `render/`, since the markdown, ANSI, and charcell targets
//! build on it too.
mod html;
pub use html::*;
#[cfg(feature = "template_serde")]
mod template;
#[cfg(feature = "template_serde")]
pub use template::*;
mod html_utils;
pub(crate) use html_utils::*;
