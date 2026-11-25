//! What extends a parsed tree, and the formats whose lowering is this crate's.
//!
//! A document is read by `beet_scene`, whose `ScenePlugin` registers the base
//! formats; [`ParsePlugin`] installs it, names [`DocMeta`] as what a
//! document's frontmatter declares, adds the formats that need styles (the
//! office formats, mermaid), and runs the [`PostParseTree`] steps (tables,
//! styles, highlighting) every parse ends in.
//!
//! [`DocMeta`]: crate::prelude::DocMeta
//! [`PostParseTree`]: beet_core::prelude::PostParseTree
#[cfg(feature = "scene")]
mod bsx;
#[cfg(feature = "scene")]
pub use bsx::*;
#[cfg(feature = "mermaid")]
mod mermaid;
#[cfg(feature = "mermaid")]
pub use mermaid::*;
mod parse_plugin;
pub use parse_plugin::*;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
mod syntax_highlighting;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
pub use syntax_highlighting::*;
