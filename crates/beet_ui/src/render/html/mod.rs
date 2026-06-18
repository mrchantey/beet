//! HTML rendering: the [`HtmlRenderer`] and its utilities, the reactive-render
//! injection (`reactive_html_render`), the template-serde wiring, and the
//! thin-client `reactivity.js` runtime served to the browser.
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
#[cfg(all(feature = "bsx", feature = "json"))]
mod reactive_html_render;
#[cfg(all(feature = "bsx", feature = "json"))]
pub use reactive_html_render::InsertReactive;
#[cfg(all(feature = "bsx", feature = "json"))]
pub use reactive_html_render::REACTIVITY_JS;
#[cfg(all(feature = "bsx", feature = "json"))]
pub use reactive_html_render::REACTIVITY_SRC;
mod html_utils;
pub use html_utils::*;
