#[cfg(feature = "style")]
mod charcell;
#[cfg(feature = "style")]
pub use charcell::*;
// the html-rendering target and its utilities; the shared
// `node_walker`/`node_renderer` substrate
// (also used by markdown/ansi/charcell) stays here in `render/`.
mod html;
pub use html::*;
// the DOM render target: the browser's document as a sink over the same walk,
// painted once and patched each frame; there is nothing to paint into
// anywhere else, so it is target-gated rather than featured.
#[cfg(target_arch = "wasm32")]
mod dom;
#[cfg(target_arch = "wasm32")]
pub use dom::*;
// the served page's pre-boot pieces: rendered natively into the page, read
// back by the browser world once it has adopted it.
mod pre_boot;
pub use pre_boot::*;
mod style_map;
pub use style_map::*;
mod media;
pub use media::*;
mod plaintext;
pub use plaintext::*;
mod node_renderer;
pub use node_renderer::*;
mod node_walker;
pub use node_walker::*;
mod text_render_state;
pub use text_render_state::*;
mod markdown;
pub use markdown::*;
#[cfg(feature = "style")]
mod ansi_term;
#[cfg(feature = "style")]
pub use ansi_term::*;
