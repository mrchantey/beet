//! The document formats that build this crate's own types, and what extends a
//! parsed tree.
//!
//! A format that builds a plain scene (elements, values, the components a
//! document names) is `beet_scene`'s, in its `formats` module; one that builds
//! a UI type, a style rule, a diagram, an office file's looks, is here, since
//! `beet_scene` sits below this crate. Both register into the one
//! `DocParsers` registry, so a document never knows which crate read it.
//!
//! A document is read by `beet_scene`, whose `ScenePlugin` registers the base
//! formats; [`ParsePlugin`] installs it, names [`DocMeta`] as what a
//! document's frontmatter declares, adds the formats that need styles, and
//! runs the [`PostParseTree`] steps (tables, styles, highlighting) every parse
//! ends in.
//!
//! - `bsx`: the BSX defaults and event scripts this crate's elements read.
//! - `mermaid`: diagrams, read from a fence or a file and drawn per sink.
//! - [`ooxml`]: Word files, workbooks and slide decks, read and written.
//! - `syntax_highlighting`: a code sample's tokens, as style.
//!
//! [`DocMeta`]: crate::prelude::DocMeta
//! [`PostParseTree`]: beet_core::prelude::PostParseTree
#[cfg(feature = "scene")]
mod bsx;
#[cfg(feature = "scene")]
pub use bsx::*;
#[cfg(feature = "mermaid")]
mod mermaid;
#[cfg(feature = "ooxml")]
pub mod ooxml;
#[cfg(feature = "mermaid")]
pub use mermaid::*;
#[cfg(feature = "ooxml")]
pub use ooxml::*;
mod parse_plugin;
pub use parse_plugin::*;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
mod syntax_highlighting;
#[cfg(all(feature = "syntax_highlighting", not(target_arch = "wasm32")))]
pub use syntax_highlighting::*;
