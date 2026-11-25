//! HTML rendering: the [`HtmlRenderer`], the
//! [`XmlRenderer`], plus the template-serde wiring.
//!
//! The shared walk/serialize substrate (`node_walker`, `render_context`) lives
//! one level up in `render/`, since the markdown, ANSI, and charcell targets
//! build on it too.
mod html;
pub use html::*;
#[cfg(feature = "scene")]
mod template;
#[cfg(feature = "scene")]
pub use template::*;
mod xml;
pub use xml::*;
