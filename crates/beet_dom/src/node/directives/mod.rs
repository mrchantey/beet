//! Directives inside RSX trees instruct beet to perform certain actions with the nodes.
//!
//! Directives are special attributes or components that modify how nodes are processed,
//! including client islands, style scoping, and language-specific transformations.

mod bsx_directives;
mod code_node;
mod lang_node;
mod rsx_directives;
mod style_scope;
mod template_directive;
mod web_directives;

pub use bsx_directives::*;
pub use code_node::*;
pub use lang_node::*;
pub use rsx_directives::*;
pub use style_scope::*;
pub use template_directive::*;
pub use web_directives::*;

/// Client island types for hydration (requires `bevy_scene` feature).
#[cfg(feature = "bevy_scene")]
pub mod client_island;
#[cfg(feature = "bevy_scene")]
pub use client_island::*;
